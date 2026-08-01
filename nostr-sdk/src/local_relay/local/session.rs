// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use negentropy::{Negentropy, NegentropyStorageVector};
use nostr::event::{Event, Kind};
use nostr::filter::Filter;
use nostr::key::PublicKey;
use nostr::message::SubscriptionId;
use nostr::nips::nip42;
use nostr::types::Timestamp;
use nostr::types::url::RelayUrl;

pub(super) enum RateLimiterResponse {
    Allowed,
    Limited,
}

#[derive(Default)]
pub(super) struct Nip42Session {
    /// Is authenticated
    pub public_key: Option<PublicKey>,
    /// Challenges
    pub challenges: HashSet<String>,
}

impl Nip42Session {
    /// Get or generate challenge
    pub fn generate_challenge(&mut self) -> String {
        // TODO: alternatives?

        // Too many challenges without reply
        if self.challenges.len() > 20 {
            // Clean to avoid possible attack where client never complete auth
            self.challenges.clear();
        }

        let challenge: String = SubscriptionId::generate().to_string();
        self.challenges.insert(challenge.clone());
        challenge
    }

    #[inline]
    pub fn is_authenticated(&self) -> bool {
        self.public_key.is_some()
    }

    pub fn check_challenge(&mut self, event: &Event, relay_url: &RelayUrl) -> Result<(), String> {
        // Authentication must be bound to the NIP-42 kind, this relay, and this connection.
        if event.kind != Kind::Authentication {
            return Err(String::from("invalid authentication event kind"));
        }

        match event.tags.challenge() {
            Some(challenge) => {
                if !self.challenges.contains(&challenge) {
                    return Err(String::from("received invalid challenge"));
                }

                // Check created_at
                let now = Timestamp::now();
                let diff: u64 = now.as_secs().abs_diff(event.created_at.as_secs());
                if diff > 120 {
                    return Err(String::from("challenge is too old (max allowed 2 min)"));
                }

                // Verify event
                event.verify().map_err(|e| e.to_string())?;

                if !nip42::is_valid_auth_event(event, relay_url, &challenge) {
                    return Err(String::from("invalid authentication event"));
                }

                // Consume only after every check, so malformed replies cannot invalidate it.
                self.challenges.remove(&challenge);

                // Mark as authenticated
                self.public_key = Some(event.pubkey);

                Ok(())
            }
            None => Err(String::from("challenge not found")),
        }
    }
}

pub(super) struct Session<'a> {
    pub subscriptions: HashMap<SubscriptionId, Vec<Filter>>,
    pub negentropy_subscription: HashMap<SubscriptionId, Negentropy<'a, NegentropyStorageVector>>,
    pub nip42: Nip42Session,
    pub tokens: Tokens,
}

impl Session<'_> {
    const MIN: Duration = Duration::from_secs(60);

    fn calculate_elapsed_time(&self, now: Instant, last: Instant) -> Duration {
        let mut elapsed_time: Duration = now - last;

        if elapsed_time > Self::MIN {
            elapsed_time = Self::MIN;
        }

        elapsed_time
    }

    pub fn check_rate_limit(&mut self, max_per_minute: u32) -> RateLimiterResponse {
        match self.tokens.last {
            Some(last) => {
                let now: Instant = Instant::now();
                let elapsed_time: Duration = self.calculate_elapsed_time(now, last);

                self.tokens
                    .calculate_new_tokens(max_per_minute, elapsed_time);

                if self.tokens.count == 0 {
                    return RateLimiterResponse::Limited;
                }

                self.tokens.last = Some(now);

                RateLimiterResponse::Allowed
            }
            None => {
                self.tokens.last = Some(Instant::now());
                RateLimiterResponse::Allowed
            }
        }
    }
}

/// Tokens to keep track of session limits
pub(super) struct Tokens {
    pub count: u32,
    pub last: Option<Instant>,
}

impl Tokens {
    #[inline]
    pub fn new(tokens: u32) -> Self {
        Self {
            count: tokens,
            last: None,
        }
    }

    fn calculate_new_tokens(&mut self, max_per_minute: u32, elapsed_time: Duration) {
        let percent: f32 = (elapsed_time.as_secs() as f32) / 60.0;
        let new_tokens: u32 = (percent * max_per_minute as f32).floor() as u32;

        self.count = self.count.saturating_add(new_tokens);

        self.count = self.count.saturating_sub(1);

        if self.count >= max_per_minute {
            self.count = max_per_minute.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use nostr::event::{EventBuilder, FinalizeEvent, IntoEventBuilder, Tag};
    use nostr::key::Keys;

    use super::*;

    #[test]
    fn authentication_rejects_signatures_from_other_event_kinds() {
        let keys = Keys::generate();
        let mut session = Nip42Session::default();
        let challenge = session.generate_challenge();
        let relay_url = RelayUrl::parse("ws://127.0.0.1:8080").unwrap();
        let event = EventBuilder::new(Kind::TextNote, "unrelated")
            .tag(Tag::custom("challenge", [challenge.as_str()]))
            .finalize(&keys)
            .unwrap();

        let error = session.check_challenge(&event, &relay_url).unwrap_err();
        assert_eq!(error, "invalid authentication event kind");
        assert!(session.challenges.contains(&challenge));
        assert!(!session.is_authenticated());
    }

    #[test]
    fn authentication_rejects_events_for_another_relay() {
        let keys = Keys::generate();
        let mut session = Nip42Session::default();
        let challenge = session.generate_challenge();
        let expected_relay = RelayUrl::parse("ws://127.0.0.1:8080").unwrap();
        let other_relay = RelayUrl::parse("wss://attacker.example.com").unwrap();
        let event = nip42::ClientAuthentication::new(challenge.clone(), other_relay)
            .into_event_builder()
            .finalize(&keys)
            .unwrap();

        let error = session
            .check_challenge(&event, &expected_relay)
            .unwrap_err();
        assert_eq!(error, "invalid authentication event");
        assert!(session.challenges.contains(&challenge));
        assert!(!session.is_authenticated());
    }
}

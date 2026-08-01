// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use nostr::event::Event;
use nostr::filter::Filter;
use nostr::types::{RelayUrl, RelayUrlArg};
use nostr_database::SaveEventStatus;
use tokio::io::{AsyncRead, AsyncWrite};

mod inner;
mod session;
mod util;

use self::inner::InnerLocalRelay;
use super::builder::LocalRelayBuilder;
use crate::client::{Output, SyncSummary};
use crate::error::Error;
use crate::relay::SyncOptions;

/// A local nostr relay
///
/// This is automatically shutdown when all instances/clones are dropped!
#[derive(Debug)]
pub struct LocalRelay {
    inner: InnerLocalRelay,
    // Keep track of the atomic reference count to know when shutdown the relay.
    atomic_counter: Arc<AtomicUsize>,
}

impl Clone for LocalRelay {
    fn clone(&self) -> Self {
        self.atomic_counter.fetch_add(1, Ordering::SeqCst);

        Self {
            inner: self.inner.clone(),
            atomic_counter: self.atomic_counter.clone(),
        }
    }
}

impl Drop for LocalRelay {
    fn drop(&mut self) {
        // Shutdown exactly once when the last handle is dropped.
        if self.atomic_counter.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.shutdown();
        }
    }
}

impl Default for LocalRelay {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl LocalRelay {
    /// Create a new local relay with the default configuration.
    ///
    /// Use [`LocalRelay::builder`] for customizing it!
    #[inline]
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Create a new local relay builder
    #[inline]
    pub fn builder() -> LocalRelayBuilder {
        LocalRelayBuilder::default()
    }

    #[inline]
    pub(super) fn from_builder(builder: LocalRelayBuilder) -> Self {
        Self {
            inner: InnerLocalRelay::new(builder),
            atomic_counter: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Run the local relay
    #[inline]
    pub async fn run(&self) -> Result<(), Error> {
        self.inner.run().await?;
        Ok(())
    }

    /// Get url
    #[inline]
    pub async fn url(&self) -> RelayUrl {
        self.inner.url().await
    }

    /// Sync events with other relay(s).
    #[inline]
    pub async fn sync_with<'a, I, U>(
        &self,
        urls: I,
        filter: Filter,
        opts: SyncOptions,
    ) -> Result<Output<SyncSummary>, Error>
    where
        I: IntoIterator<Item = U>,
        U: Into<RelayUrlArg<'a>>,
    {
        self.inner.sync_with(urls, filter, opts).await
    }

    /// Send event to subscribers
    ///
    /// Return `true` if the event is successfully sent.
    ///
    /// This method doesn't save the event into the database!
    /// It's intended to be used ONLY when the database is shared with other apps (i.e. with the nostr-sdk `Client`).
    pub fn notify_event(&self, event: Event) -> bool {
        if event.verify().is_err() {
            return false;
        }

        self.inner.notify_event(event)
    }

    /// Save the event to the database and, if success, notify the subscribers.
    pub async fn add_event(&self, event: Event) -> Result<SaveEventStatus, Error> {
        event.verify()?;

        let status = self.inner.save_event(&event).await?;

        if status.is_success() {
            self.inner.notify_event(event);
        }

        Ok(status)
    }

    /// Shutdown relay
    #[inline]
    pub fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Pass an already upgraded stream
    pub async fn take_connection<S>(&self, stream: S, addr: SocketAddr) -> Result<(), Error>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        self.inner.handle_upgraded_connection(stream, addr).await
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::time::Duration;

    use async_wsocket::{ConnectionMode, Message, Url, WebSocket};
    use futures::{SinkExt, StreamExt};
    use nostr::event::{EventBuilder, FinalizeEvent, Kind};
    use nostr::filter::Filter;
    use nostr::key::Keys;
    use nostr::message::{MachineReadablePrefix, RelayMessage};
    use tokio::time;

    use super::*;
    use crate::local_relay::{QueryPolicy, QueryPolicyResult};

    #[derive(Debug)]
    struct RejectQueries;

    impl QueryPolicy for RejectQueries {
        fn admit_query<'a>(
            &'a self,
            _query: &'a mut Filter,
            _addr: &'a SocketAddr,
        ) -> Pin<Box<dyn Future<Output = QueryPolicyResult> + Send + 'a>> {
            Box::pin(async {
                QueryPolicyResult::reject(MachineReadablePrefix::Blocked, "query rejected")
            })
        }
    }

    #[tokio::test]
    async fn add_event_rejects_unverified_events() {
        let relay = LocalRelay::new();
        let keys = Keys::generate();
        let mut event = EventBuilder::new(Kind::TextNote, "original")
            .finalize(&keys)
            .unwrap();
        event.content = String::from("forged");

        let err = relay.add_event(event).await.unwrap_err();
        assert_eq!(err.kind(), crate::error::ErrorKind::Protocol);
    }

    #[test]
    fn notify_event_rejects_unverified_events() {
        let relay = LocalRelay::new();
        let keys = Keys::generate();
        let mut event = EventBuilder::new(Kind::TextNote, "original")
            .finalize(&keys)
            .unwrap();
        event.content = String::from("forged");

        assert!(!relay.notify_event(event));
    }

    #[tokio::test]
    async fn test_malformed_client_message_does_not_close_connection() {
        let relay = LocalRelay::new();
        relay.run().await.unwrap();

        let url = Url::parse(relay.url().await.as_str()).unwrap();
        let mut socket = WebSocket::connect(&url, &ConnectionMode::direct())
            .await
            .unwrap();

        socket
            .send(Message::Text(
                r#"["REQ","short-author",{"authors":["deadbeef"]}]"#.to_owned(),
            ))
            .await
            .unwrap();
        socket
            .send(Message::Text(r#"["REQ","valid",{}]"#.to_owned()))
            .await
            .unwrap();

        time::timeout(Duration::from_secs(1), async {
            let mut received_notice = false;

            loop {
                let message = socket
                    .next()
                    .await
                    .expect("WebSocket connection terminated")
                    .unwrap();

                if let Message::Text(json) = message {
                    match RelayMessage::from_json(json.as_bytes()).unwrap() {
                        RelayMessage::Notice(..) => received_notice = true,
                        RelayMessage::EndOfStoredEvents(subscription_id)
                            if subscription_id.as_str() == "valid" =>
                        {
                            assert!(received_notice);
                            break;
                        }
                        _ => {}
                    }
                }
            }
        })
        .await
        .expect("timed out waiting for EOSE");
    }

    #[tokio::test]
    async fn test_nip42_read_auth_is_required_for_count() {
        let relay = LocalRelay::builder()
            .nip42(crate::local_relay::LocalRelayBuilderNip42::read())
            .build();
        relay.run().await.unwrap();

        let url = Url::parse(relay.url().await.as_str()).unwrap();
        let mut socket = WebSocket::connect(&url, &ConnectionMode::direct())
            .await
            .unwrap();
        socket
            .send(Message::Text(r#"["COUNT","count",{}]"#.to_owned()))
            .await
            .unwrap();

        let auth = socket.next().await.unwrap().unwrap();
        let Message::Text(auth) = auth else {
            panic!("unexpected websocket message");
        };
        assert!(matches!(
            RelayMessage::from_json(auth.as_bytes()).unwrap(),
            RelayMessage::Auth { .. }
        ));

        let closed = socket.next().await.unwrap().unwrap();
        let Message::Text(closed) = closed else {
            panic!("unexpected websocket message");
        };
        assert!(matches!(
            RelayMessage::from_json(closed.as_bytes()).unwrap(),
            RelayMessage::Closed {
                subscription_id,
                message,
            } if subscription_id.as_str() == "count"
                && message.starts_with("auth-required:")
        ));
    }

    #[tokio::test]
    async fn test_nip42_read_auth_is_required_for_negentropy() {
        let relay = LocalRelay::builder()
            .nip42(crate::local_relay::LocalRelayBuilderNip42::read())
            .build();
        relay.run().await.unwrap();

        let url = Url::parse(relay.url().await.as_str()).unwrap();
        let mut socket = WebSocket::connect(&url, &ConnectionMode::direct())
            .await
            .unwrap();
        socket
            .send(Message::Text(r#"["NEG-OPEN","neg",{},""]"#.to_owned()))
            .await
            .unwrap();

        let auth = socket.next().await.unwrap().unwrap();
        let Message::Text(auth) = auth else {
            panic!("unexpected websocket message");
        };
        assert!(matches!(
            RelayMessage::from_json(auth.as_bytes()).unwrap(),
            RelayMessage::Auth { .. }
        ));

        let neg_err = socket.next().await.unwrap().unwrap();
        let Message::Text(neg_err) = neg_err else {
            panic!("unexpected websocket message");
        };
        assert!(matches!(
            RelayMessage::from_json(neg_err.as_bytes()).unwrap(),
            RelayMessage::NegErr {
                subscription_id,
                message,
            } if subscription_id.as_str() == "neg"
                && message.starts_with("auth-required:")
        ));
    }

    #[tokio::test]
    async fn test_query_policy_is_applied_to_count_and_negentropy() {
        let relay = LocalRelay::builder().query_policy(RejectQueries).build();
        relay.run().await.unwrap();

        let url = Url::parse(relay.url().await.as_str()).unwrap();
        let mut socket = WebSocket::connect(&url, &ConnectionMode::direct())
            .await
            .unwrap();
        socket
            .send(Message::Text(r#"["COUNT","count",{}]"#.to_owned()))
            .await
            .unwrap();

        let closed = socket.next().await.unwrap().unwrap();
        let Message::Text(closed) = closed else {
            panic!("unexpected websocket message");
        };
        assert!(matches!(
            RelayMessage::from_json(closed.as_bytes()).unwrap(),
            RelayMessage::Closed {
                subscription_id,
                message,
            } if subscription_id.as_str() == "count"
                && message == "blocked: query rejected"
        ));

        socket
            .send(Message::Text(r#"["NEG-OPEN","neg",{},""]"#.to_owned()))
            .await
            .unwrap();

        let neg_err = socket.next().await.unwrap().unwrap();
        let Message::Text(neg_err) = neg_err else {
            panic!("unexpected websocket message");
        };
        assert!(matches!(
            RelayMessage::from_json(neg_err.as_bytes()).unwrap(),
            RelayMessage::NegErr {
                subscription_id,
                message,
            } if subscription_id.as_str() == "neg"
                && message == "blocked: query rejected"
        ));
    }

    #[tokio::test]
    async fn test_negentropy_subscription_limit() {
        let relay = LocalRelay::builder()
            .max_negentropy_subscriptions(0)
            .build();
        relay.run().await.unwrap();

        let url = Url::parse(relay.url().await.as_str()).unwrap();
        let mut socket = WebSocket::connect(&url, &ConnectionMode::direct())
            .await
            .unwrap();
        socket
            .send(Message::Text(r#"["NEG-OPEN","neg",{},""]"#.to_owned()))
            .await
            .unwrap();

        let neg_err = socket.next().await.unwrap().unwrap();
        let Message::Text(neg_err) = neg_err else {
            panic!("unexpected websocket message");
        };
        assert!(matches!(
            RelayMessage::from_json(neg_err.as_bytes()).unwrap(),
            RelayMessage::NegErr {
                subscription_id,
                message,
            } if subscription_id.as_str() == "neg"
                && message.starts_with("rate-limited:")
        ));
    }

    #[tokio::test]
    async fn test_binary_messages_are_rate_limited() {
        let relay = LocalRelay::builder().messages_per_minute(1).build();
        relay.run().await.unwrap();

        let url = Url::parse(relay.url().await.as_str()).unwrap();
        let mut socket = WebSocket::connect(&url, &ConnectionMode::direct())
            .await
            .unwrap();

        socket.send(Message::Binary(vec![1].into())).await.unwrap();
        let notice = time::timeout(Duration::from_secs(1), socket.next())
            .await
            .expect("relay did not answer the first binary message")
            .unwrap()
            .unwrap();
        assert!(matches!(
            notice,
            Message::Text(json)
                if matches!(
                    RelayMessage::from_json(json.as_bytes()).unwrap(),
                    RelayMessage::Notice(..)
                )
        ));

        socket.send(Message::Binary(vec![2].into())).await.unwrap();
        let closed = time::timeout(Duration::from_secs(1), socket.next())
            .await
            .expect("connection did not close after rate limit");
        assert!(matches!(
            closed,
            None | Some(Ok(Message::Close(..))) | Some(Err(..))
        ));
    }

    #[tokio::test]
    async fn test_shutdown() {
        let relay = LocalRelay::new();

        assert!(!relay.inner.is_running());

        relay.run().await.unwrap();

        time::sleep(Duration::from_secs(1)).await;

        assert!(relay.inner.is_running());

        relay.shutdown();

        time::sleep(Duration::from_millis(100)).await;

        assert!(!relay.inner.is_running());
    }

    #[tokio::test]
    async fn test_shutdown_on_drop() {
        let inner: InnerLocalRelay = {
            let relay: LocalRelay = LocalRelay::new();

            assert!(!relay.inner.is_running());

            relay.run().await.unwrap();

            time::sleep(Duration::from_secs(1)).await;

            assert!(relay.inner.is_running());

            // Clone the inner relay
            let inner: InnerLocalRelay = relay.inner.clone();

            {
                let r2: LocalRelay = relay.clone();
                tokio::spawn(async move {
                    assert_eq!(r2.atomic_counter.load(Ordering::SeqCst), 2);

                    time::sleep(Duration::from_secs(1)).await;

                    // r2 dropped here
                });
            }

            time::sleep(Duration::from_secs(2)).await;

            assert_eq!(relay.atomic_counter.load(Ordering::SeqCst), 1);

            inner
        }; // relay dropped here

        time::sleep(Duration::from_secs(1)).await;

        assert!(!inner.is_running());
    }
}

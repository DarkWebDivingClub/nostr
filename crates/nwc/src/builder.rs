//! Nostr Wallet Connect builder

use std::time::Duration;

use nostr::nips::nip47::{Nip47Ciphers, NostrWalletConnectUri};
use nostr_sdk::monitor::Monitor;
use nostr_sdk::relay::RelayOptions;

use crate::NostrWalletConnect;

/// Default timeout
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Nostr Wallet Connect builder
#[derive(Debug, Clone)]
pub struct NostrWalletConnectBuilder {
    /// NWC URI.
    pub uri: NostrWalletConnectUri,
    /// Requests timeout.
    pub timeout: Duration,
    /// Relay monitor.
    pub monitor: Option<Monitor>,
    /// Relay options.
    ///
    /// See [`RelayOptions`] for more details.
    pub relay: RelayOptions,
    /// NIP-47 cipher used for request encryption and response decryption.
    ///
    /// When `None`, the client fetches the wallet info event to determine the
    /// cipher.
    pub cipher: Option<Nip47Ciphers>,
}

impl NostrWalletConnectBuilder {
    /// Construct a new Nostr Wallet Connect client builder.
    pub fn new(uri: NostrWalletConnectUri) -> Self {
        Self {
            uri,
            timeout: DEFAULT_TIMEOUT,
            monitor: None,
            relay: RelayOptions::default(),
            cipher: None,
        }
    }

    /// Set default requests timeout (default: 10 secs)
    #[inline]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the relay monitor
    #[inline]
    pub fn monitor(mut self, monitor: Monitor) -> Self {
        self.monitor = Some(monitor);
        self
    }

    /// Set relay options
    #[inline]
    pub fn relay(mut self, opts: RelayOptions) -> Self {
        self.relay = opts;
        self
    }

    /// Configure legacy NIP04 cipher regardless of wallet support.
    #[inline]
    pub fn force_nip04(mut self) -> Self {
        self.cipher = Some(Nip47Ciphers::NIP04);
        self
    }

    /// Configure nip44 v2 cipher regardless of wallet support.
    #[inline]
    pub fn force_nip44_v2(mut self) -> Self {
        self.cipher = Some(Nip47Ciphers::NIP44V2);
        self
    }

    /// Build [`NostrWalletConnect`] client.
    #[inline]
    pub fn build(self) -> NostrWalletConnect {
        NostrWalletConnect::from_builder(self)
    }
}

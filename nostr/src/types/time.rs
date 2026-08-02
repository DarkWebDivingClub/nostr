// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! Time

use core::fmt;
use core::num::{ParseIntError, TryFromIntError};
use core::ops::{Add, Sub};
use core::str::{self, FromStr};
use core::time::Duration;

use universal_time::{SystemTime, UNIX_EPOCH};

/// Unix timestamp in seconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Construct from seconds
    #[inline]
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs)
    }

    /// Compose `0` timestamp
    #[inline]
    pub const fn zero() -> Self {
        Self::from_secs(0)
    }

    /// The minimum representable timestamp
    #[inline]
    pub const fn min() -> Self {
        Self::from_secs(u64::MIN)
    }

    /// The maximum representable timestamp
    #[inline]
    pub const fn max() -> Self {
        Self::from_secs(u64::MAX)
    }

    /// Get UNIX timestamp
    pub fn now() -> Self {
        let ts: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self::from_secs(ts)
    }

    /// Get timestamp as seconds
    #[inline]
    pub const fn as_secs(&self) -> u64 {
        self.0
    }

    /// Check if timestamp is `0`
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Default for Timestamp {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl From<u64> for Timestamp {
    fn from(secs: u64) -> Self {
        Self::from_secs(secs)
    }
}

impl TryFrom<i64> for Timestamp {
    type Error = TryFromIntError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        let secs: u64 = value.try_into()?;
        Ok(Self::from_secs(secs))
    }
}

impl FromStr for Timestamp {
    type Err = ParseIntError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_secs(s.parse::<u64>()?))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add<Timestamp> for Timestamp {
    type Output = Self;
    fn add(self, rhs: Timestamp) -> Self::Output {
        Self::from_secs(self.0.saturating_add(rhs.as_secs()))
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Self;
    fn sub(self, rhs: Timestamp) -> Self::Output {
        Self::from_secs(self.0.saturating_sub(rhs.as_secs()))
    }
}

impl Add<Duration> for Timestamp {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self::from_secs(self.0.saturating_add(rhs.as_secs()))
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self::from_secs(self.0.saturating_sub(rhs.as_secs()))
    }
}

impl Add<u64> for Timestamp {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self::from_secs(self.0.saturating_add(rhs))
    }
}

impl Sub<u64> for Timestamp {
    type Output = Self;

    fn sub(self, rhs: u64) -> Self::Output {
        Self::from_secs(self.0.saturating_sub(rhs))
    }
}

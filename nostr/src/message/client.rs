// Copyright (c) 2021 Paul Miller
// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! Client messages

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use serde::de::{self, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::SubscriptionId;
use crate::event::Event;
use crate::filter::Filter;
use crate::util::impl_json_methods;

/// Messages sent by clients, received by relays
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClientMessage<'a> {
    /// Event
    Event(Cow<'a, Event>),
    /// Req
    Req {
        /// Subscription ID
        subscription_id: Cow<'a, SubscriptionId>,
        /// Filter
        filters: Vec<Cow<'a, Filter>>,
    },
    /// Count
    ///
    /// <https://github.com/nostr-protocol/nips/blob/master/45.md>
    Count {
        /// Subscription ID
        subscription_id: Cow<'a, SubscriptionId>,
        /// Filter
        filter: Cow<'a, Filter>,
    },
    /// Close
    Close(Cow<'a, SubscriptionId>),
    /// Auth
    Auth(Cow<'a, Event>),
    /// Negentropy Open
    NegOpen {
        /// Subscription ID
        subscription_id: Cow<'a, SubscriptionId>,
        /// Filter
        filter: Cow<'a, Filter>,
        /// Initial message (hex)
        initial_message: Cow<'a, str>,
    },
    /// Negentropy Message
    NegMsg {
        /// Subscription ID
        subscription_id: Cow<'a, SubscriptionId>,
        /// Message
        message: Cow<'a, str>,
    },
    /// Negentropy Close
    NegClose {
        /// Subscription ID
        subscription_id: Cow<'a, SubscriptionId>,
    },
}

impl ClientMessage<'_> {
    /// Create `EVENT` message
    #[inline]
    pub fn event(event: Event) -> Self {
        Self::Event(Cow::Owned(event))
    }

    /// Create `REQ` message
    #[inline]
    pub fn req<T>(subscription_id: SubscriptionId, filters: T) -> Self
    where
        T: Into<Vec<Filter>>,
    {
        Self::Req {
            subscription_id: Cow::Owned(subscription_id),
            filters: filters.into().into_iter().map(Cow::Owned).collect(),
        }
    }

    /// Create `COUNT` message
    #[inline]
    pub fn count(subscription_id: SubscriptionId, filter: Filter) -> Self {
        Self::Count {
            subscription_id: Cow::Owned(subscription_id),
            filter: Cow::Owned(filter),
        }
    }

    /// Create new `CLOSE` message
    #[inline]
    pub fn close(subscription_id: SubscriptionId) -> Self {
        Self::Close(Cow::Owned(subscription_id))
    }

    /// Create `AUTH` message
    #[inline]
    pub fn auth(event: Event) -> Self {
        Self::Auth(Cow::Owned(event))
    }

    /// Create new `NEG-OPEN` message
    pub fn neg_open(
        subscription_id: SubscriptionId,
        filter: Filter,
        initial_message: String,
    ) -> Self {
        Self::NegOpen {
            subscription_id: Cow::Owned(subscription_id),
            filter: Cow::Owned(filter),
            initial_message: Cow::Owned(initial_message),
        }
    }

    /// Check if is an `EVENT` message
    #[inline]
    pub fn is_event(&self) -> bool {
        matches!(self, ClientMessage::Event(_))
    }

    /// Check if is an `REQ` message
    #[inline]
    pub fn is_req(&self) -> bool {
        matches!(self, ClientMessage::Req { .. })
    }

    /// Check if is an `CLOSE` message
    #[inline]
    pub fn is_close(&self) -> bool {
        matches!(self, ClientMessage::Close(_))
    }

    /// Check if is an `AUTH` message
    #[inline]
    pub fn is_auth(&self) -> bool {
        matches!(self, ClientMessage::Auth(_))
    }

    /// Number of elements in the JSON array form.
    ///
    /// Matched exhaustively on purpose: a new variant must state its own
    /// length rather than inherit a default that may not fit.
    fn len(&self) -> usize {
        match self {
            Self::Event(..) | Self::Close(..) | Self::Auth(..) | Self::NegClose { .. } => 2,
            Self::Count { .. } | Self::NegMsg { .. } => 3,
            Self::Req { filters, .. } => 2 + filters.len(),
            Self::NegOpen { .. } => 4,
        }
    }
}

impl Serialize for ClientMessage<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Write the array elements straight to the serializer. Building a
        // `serde_json::Value` first would allocate for every element, and for
        // an `EVENT` message that means the whole event too.
        let mut seq = serializer.serialize_seq(Some(self.len()))?;

        match self {
            Self::Event(event) => {
                seq.serialize_element("EVENT")?;
                seq.serialize_element(event)?;
            }
            Self::Req {
                subscription_id,
                filters,
            } => {
                seq.serialize_element("REQ")?;
                seq.serialize_element(subscription_id)?;
                for filter in filters {
                    seq.serialize_element(filter)?;
                }
            }
            Self::Count {
                subscription_id,
                filter,
            } => {
                seq.serialize_element("COUNT")?;
                seq.serialize_element(subscription_id)?;
                seq.serialize_element(filter)?;
            }
            Self::Close(subscription_id) => {
                seq.serialize_element("CLOSE")?;
                seq.serialize_element(subscription_id)?;
            }
            Self::Auth(event) => {
                seq.serialize_element("AUTH")?;
                seq.serialize_element(event)?;
            }
            Self::NegOpen {
                subscription_id,
                filter,
                initial_message,
            } => {
                seq.serialize_element("NEG-OPEN")?;
                seq.serialize_element(subscription_id)?;
                seq.serialize_element(filter)?;
                seq.serialize_element(initial_message)?;
            }
            Self::NegMsg {
                subscription_id,
                message,
            } => {
                seq.serialize_element("NEG-MSG")?;
                seq.serialize_element(subscription_id)?;
                seq.serialize_element(message)?;
            }
            Self::NegClose { subscription_id } => {
                seq.serialize_element("NEG-CLOSE")?;
                seq.serialize_element(subscription_id)?;
            }
        }

        seq.end()
    }
}

impl<'de> Deserialize<'de> for ClientMessage<'_> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ClientMessageVisitor)
    }
}

struct ClientMessageVisitor;

impl<'de> Visitor<'de> for ClientMessageVisitor {
    type Value = ClientMessage<'static>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a client message array")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        fn malformed<E>() -> E
        where
            E: de::Error,
        {
            E::custom("invalid message format")
        }

        macro_rules! next {
            () => {
                seq.next_element()?.ok_or_else(malformed)?
            };
        }

        let message_type: String = next!();

        let message: ClientMessage<'static> = match message_type.as_str() {
            // ["EVENT", <event JSON>]
            "EVENT" => ClientMessage::Event(Cow::Owned(next!())),
            // ["REQ", <subscription_id>, <filter JSON>, <filter JSON>, ...]
            "REQ" => {
                let subscription_id: SubscriptionId = next!();
                let mut filters: Vec<Cow<'static, Filter>> = Vec::new();
                while let Some(filter) = seq.next_element::<Filter>()? {
                    filters.push(Cow::Owned(filter));
                }
                if filters.is_empty() {
                    return Err(malformed());
                }
                return Ok(ClientMessage::Req {
                    subscription_id: Cow::Owned(subscription_id),
                    filters,
                });
            }
            // ["COUNT", <subscription_id>, <filter JSON>]
            "COUNT" => ClientMessage::Count {
                subscription_id: Cow::Owned(next!()),
                filter: Cow::Owned(next!()),
            },
            // ["CLOSE", <subscription_id>]
            "CLOSE" => ClientMessage::Close(Cow::Owned(next!())),
            // ["AUTH", <event JSON>]
            "AUTH" => ClientMessage::Auth(Cow::Owned(next!())),
            // ["NEG-OPEN", <subscription ID string>, <filter>, <initial message>]
            "NEG-OPEN" => {
                let subscription_id: SubscriptionId = next!();
                let filter: Filter = next!();
                let initial_message: String = next!();

                ClientMessage::NegOpen {
                    subscription_id: Cow::Owned(subscription_id),
                    filter: Cow::Owned(filter),
                    initial_message: Cow::Owned(initial_message),
                }
            }
            // ["NEG-MSG", <subscription ID string>, <message, lowercase hex-encoded>]
            "NEG-MSG" => ClientMessage::NegMsg {
                subscription_id: Cow::Owned(next!()),
                message: Cow::Owned(next!()),
            },
            // ["NEG-CLOSE", <subscription ID string>]
            "NEG-CLOSE" => ClientMessage::NegClose {
                subscription_id: Cow::Owned(next!()),
            },
            _ => return Err(malformed()),
        };

        while seq.next_element::<de::IgnoredAny>()?.is_some() {}

        Ok(message)
    }
}

impl_json_methods!(ClientMessage<'_>);

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use super::*;
    use crate::error::ErrorKind;
    use crate::event::Kind;
    use crate::key::PublicKey;

    const EVENT_JSON: &str = r#"{"id":"70b10f70c1318967eddf12527799411b1a9780ad9c43858f5e5fcd45486a13a5","pubkey":"379e863e8357163b5bce5d2688dc4f1dcc2d505222fb8d74db600f30535dfdfe","created_at":1612809991,"kind":1,"tags":[],"content":"test","sig":"273a9cd5d11455590f4359500bccb7a89428262b96b3ea87a756b770964472f8c3e87f5d5e64d8d2e859a71462a3f477b554565c4f2f326cb01dd7620db71502"}"#;

    #[test]
    fn test_client_message_req() {
        let pk =
            PublicKey::from_str("379e863e8357163b5bce5d2688dc4f1dcc2d505222fb8d74db600f30535dfdfe")
                .unwrap();

        let client_req = ClientMessage::req(SubscriptionId::new("test"), Filter::new().pubkey(pk));
        assert_eq!(
            client_req.as_json(),
            r##"["REQ","test",{"#p":["379e863e8357163b5bce5d2688dc4f1dcc2d505222fb8d74db600f30535dfdfe"]}]"##
        );
    }

    #[test]
    fn test_client_message_custom_kind() {
        let client_req = ClientMessage::req(
            SubscriptionId::new("test"),
            Filter::new().kind(Kind::Custom(22)),
        );
        assert_eq!(client_req.as_json(), r##"["REQ","test",{"kinds":[22]}]"##);
    }

    /// Trailing elements are reserved for future extensions, so fixed-length
    /// variants other than `NEG-OPEN` must continue to ignore them.
    #[test]
    fn parse_trailing_elements() {
        let cases: [(&str, ClientMessage); 3] = [
            (
                r#"["COUNT","sub",{"kinds":[1]},"extra"]"#,
                ClientMessage::count(
                    SubscriptionId::new("sub"),
                    Filter::new().kind(Kind::TextNote),
                ),
            ),
            (
                r#"["CLOSE","sub",{"extra":true}]"#,
                ClientMessage::close(SubscriptionId::new("sub")),
            ),
            (
                r#"["NEG-MSG","sub","deadbeef",1,2]"#,
                ClientMessage::NegMsg {
                    subscription_id: Cow::Owned(SubscriptionId::new("sub")),
                    message: Cow::Borrowed("deadbeef"),
                },
            ),
        ];

        for (json, expected) in cases {
            assert_eq!(ClientMessage::from_json(json).unwrap(), expected, "{json}");
        }
    }

    #[test]
    fn round_trip_every_variant() {
        let event: Event = Event::from_json(EVENT_JSON).unwrap();
        let sub = || SubscriptionId::new("sub");
        let filter = || Filter::new().kind(Kind::TextNote);

        let messages: [ClientMessage; 8] = [
            ClientMessage::event(event.clone()),
            ClientMessage::req(sub(), vec![filter(), Filter::new().author(event.pubkey)]),
            ClientMessage::count(sub(), filter()),
            ClientMessage::close(sub()),
            ClientMessage::auth(event),
            ClientMessage::neg_open(sub(), filter(), String::from("deadbeef")),
            ClientMessage::NegMsg {
                subscription_id: Cow::Owned(sub()),
                message: Cow::Borrowed("deadbeef"),
            },
            ClientMessage::NegClose {
                subscription_id: Cow::Owned(sub()),
            },
        ];

        for message in messages {
            let json: String = message.as_json();
            assert_eq!(ClientMessage::from_json(&json).unwrap(), message, "{json}");
        }
    }

    #[test]
    fn parse_rejects_unknown_type_and_non_array() {
        for json in [
            r#"["NOT-A-REAL-TYPE","x"]"#,
            r#"{"type":"EVENT"}"#,
            r#""EVENT""#,
            r#"[]"#,
            r#"["REQ","sub"]"#,
            r#"["NEG-OPEN","sub",{},16,"deadbeef"]"#,
        ] {
            let err = ClientMessage::from_json(json).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::Malformed, "{json}");
        }
    }

    /// An `EVENT` message must embed the event exactly as the event serializes
    /// on its own. Round-tripping through `serde_json::Value` used to reorder
    /// the event's keys alphabetically.
    #[test]
    fn event_message_embeds_canonical_event() {
        let event: Event = Event::from_json(EVENT_JSON).unwrap();
        let message: ClientMessage = ClientMessage::event(event.clone());
        let json: String = message.as_json();

        assert!(
            json.contains(&event.as_json()),
            "event was not embedded verbatim: {json}"
        );
    }
}

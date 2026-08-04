// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

macro_rules! impl_tag_codec_conversions {
    (
        $ty:ty,
        fn parse($tag:ident) $parse_body:block
        fn to_tag(&$value:ident) $to_tag_body:block
    ) => {
        impl $ty {
            /// Parse a typed tag from a raw tag representation.
            pub fn parse<I, S>($tag: I) -> Result<Self, $crate::error::Error>
            where
                I: IntoIterator<Item = S>,
                S: AsRef<str>,
            $parse_body

            pub(crate) fn to_tag(&$value) -> $crate::event::Tag $to_tag_body
        }

        impl From<&$ty> for $crate::event::Tag {
            #[inline]
            fn from(value: &$ty) -> Self {
                value.to_tag()
            }
        }

        impl From<$ty> for $crate::event::Tag {
            #[inline]
            fn from(value: $ty) -> Self {
                value.to_tag()
            }
        }

        impl TryFrom<&$crate::event::Tag> for $ty {
            type Error = $crate::error::Error;

            #[inline]
            fn try_from(tag: &$crate::event::Tag) -> Result<Self, Self::Error> {
                <$ty>::parse(tag.as_slice())
            }
        }

        impl TryFrom<$crate::event::Tag> for $ty {
            type Error = $crate::error::Error;

            #[inline]
            fn try_from(tag: $crate::event::Tag) -> Result<Self, Self::Error> {
                <$ty>::parse(tag.as_slice())
            }
        }
    };
}

pub use impl_tag_codec_conversions;

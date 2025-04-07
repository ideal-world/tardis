mod endecode;
mod trim;
pub use endecode::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};
pub use trim::*;

/// The trait for mapping T into another type. A trait version of `Fn(T) -> Output`
pub trait Mapper<T> {
    type Output;
    fn map(value: T) -> Self::Output;
}

/// A wrapper of the mapped value of a mapper `M: Mapper<T>`.
///
/// Notice that the computation of the mapped value is **not** lazy. It will be computed when the `Mapped` value is created.
///
/// To take the inner output value, use [`Mapped::into_inner()`].
///
/// # Deserialize
/// It can be used as a type to deserialize from json.
/// ```ignore
/// #[derive(Serialize, Deserialize)]
/// struct SomeReq {
///     trimed_string: Mapped<String, Trim>,
///     base64_decoded: Mapped<String, Base64Decoded>
/// }
///
/// ```
///
/// # Combination
/// You can combinate multiple mappers into one mapper by using tuple.
/// ```ignore
/// #[derive(Serialize, Deserialize)]
/// struct SomeReq {
///     trimed_base64_decoded_string: Mapped<String, (Trim, Base64Decode)>,
/// }
/// ```
///
/// # Work with poem-openapi
/// Enable feature `web-server` to allow you use `Mapped` in poem-openapi object.
/// A `Mapped` value implements `poem-openapi`'s `Type`, `ToJSON` and `ParseFromJSON` trait.
///
#[derive(Copy)]
#[repr(transparent)]
pub struct Mapped<T, M>
where
    M: Mapper<T>,
{
    pub(crate) inner: M::Output,
}

impl<T, M> Clone for Mapped<T, M>
where
    M: Mapper<T>,
    M::Output: Clone,
{
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T, M> PartialEq<Self> for Mapped<T, M>
where
    M: Mapper<T>,
    M::Output: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T, M> Eq for Mapped<T, M>
where
    M: Mapper<T>,
    M::Output: Eq,
{
}

impl<T, M> std::hash::Hash for Mapped<T, M>
where
    M: Mapper<T>,
    M::Output: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl<T, M> Default for Mapped<T, M>
where
    M: Mapper<T>,
    T: Default,
{
    fn default() -> Self {
        Mapped::new(T::default())
    }
}

impl<T, M> Mapped<T, M>
where
    M: Mapper<T>,
{
    /// create a new mapped value
    pub fn new(value: T) -> Self {
        Mapped { inner: M::map(value) }
    }

    /// take the inner value
    pub fn into_inner(self) -> M::Output {
        self.inner
    }
}

impl<T, M> Deref for Mapped<T, M>
where
    M: Mapper<T>,
    M::Output: Deref,
{
    type Target = <M::Output as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T, M> AsRef<M::Output> for Mapped<T, M>
where
    M: Mapper<T>,
{
    fn as_ref(&self) -> &M::Output {
        &self.inner
    }
}

impl<T> Mapper<T> for () {
    type Output = T;
    fn map(value: T) -> Self::Output {
        value
    }
}

impl<T, M> Serialize for Mapped<T, M>
where
    M: Mapper<T>,
    M::Output: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, T, M> Deserialize<'de> for Mapped<T, M>
where
    M: Mapper<T>,
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Mapped::new(T::deserialize(deserializer)?))
    }
}

impl<T, M> From<T> for Mapped<T, M>
where
    M: Mapper<T>,
{
    fn from(value: T) -> Self {
        Mapped::new(value)
    }
}

impl<T, M> Debug for Mapped<T, M>
where
    M: Mapper<T>,
    M::Output: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
impl<T, M> Display for Mapped<T, M>
where
    M: Mapper<T>,
    M::Output: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

#[cfg(feature = "web-server")]
mod web_server_ext {
    use poem_openapi::types::{ParseFromJSON, ToJSON};

    use super::*;
    use crate::web::poem_openapi::types::{ParseResult, Type};
    impl<T, M> Type for Mapped<T, M>
    where
        M: Mapper<T> + Sync + Send,
        M::Output: crate::web::poem_openapi::types::Type,
    {
        const IS_REQUIRED: bool = true;

        type RawValueType = <M::Output as crate::web::poem_openapi::types::Type>::RawValueType;

        type RawElementValueType = <M::Output as crate::web::poem_openapi::types::Type>::RawElementValueType;

        fn name() -> std::borrow::Cow<'static, str> {
            M::Output::name()
        }

        fn schema_ref() -> poem_openapi::registry::MetaSchemaRef {
            M::Output::schema_ref()
        }

        fn as_raw_value(&self) -> Option<&Self::RawValueType> {
            self.inner.as_raw_value()
        }

        fn raw_element_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
            self.inner.raw_element_iter()
        }
    }

    impl<T, M> ToJSON for Mapped<T, M>
    where
        M: Mapper<T> + Sync + Send,
        M::Output: ToJSON,
    {
        fn to_json(&self) -> Option<serde_json::Value> {
            self.inner.to_json()
        }
    }

    impl<T, M> ParseFromJSON for Mapped<T, M>
    where
        M: Mapper<T> + Sync + Send,
        M::Output: ParseFromJSON,
        T: for<'de> Deserialize<'de>,
    {
        fn parse_from_json(value: Option<serde_json::Value>) -> ParseResult<Self> {
            let value = value.unwrap_or_default();
            match serde_json::from_value(value) {
                Ok(value) => Ok(Mapped::new(value)),
                Err(e) => Err(poem_openapi::types::ParseError::custom(e)),
            }
        }
    }
}

macro_rules! impl_mapper_for_tuple {
    {@$call:ident #rev $(#$argfmt:ident)* $first:literal, $($index:literal,)*; $($rev: literal,)*} => {
        impl_mapper_for_tuple!(@$call #rev $(#$argfmt)* $($index,)*; $first, $($rev,)* );
    };
    {@$call:ident #rev $(#$argfmt:ident)* ; $($rev: literal,)*} => {
        impl_mapper_for_tuple!(@$call $(#$argfmt)* $($rev,)*);
    };
    {@$call:ident #window2 $(#$argfmt:ident)* $first:literal, } => {};
    {@$call:ident #window2 $(#$argfmt:ident)* $first:literal, $second:literal, } => {
        impl_mapper_for_tuple!(
            @$call
            $(#$argfmt)*
            $first;
            $first,;
            $second, ;
            $second
        );
    };
    {@$call:ident #window2 $(#$argfmt:ident)* $first:literal, $second:literal, $($rest:literal,)*} => {
        impl_mapper_for_tuple!(
            @$call
            #window2 $(#$argfmt)*
            $first;
            $first, $second, ;
            $second, ;
            $($rest,)*
        );
    };
    {@$call:ident #window2 $(#$argfmt:ident)*  $first:literal; $($a:literal,)*; $($b:literal,)*; $last:literal, } => {
        impl_mapper_for_tuple!(
            @$call
            $(#$argfmt)*
            $first;
            $($a,)*;
            $($b,)* $last,;
            $last
        );
    };
    {@$call:ident #window2 $(#$argfmt:ident)* $first:literal; $($a:literal,)*; $($b:literal,)*; $next:literal, $($rest:literal,)+ } => {
        impl_mapper_for_tuple!(
            @$call
            #window2 $(#$argfmt)*
            $first;
            $($a,)* $next, ;
            $($b,)* $next, ;
            $($rest,)+
        );
    };
    {@gen $last:literal,  $($index:literal,)*} => {
        impl_mapper_for_tuple!(@gen $($index,)*);
        impl_mapper_for_tuple!(@item #rev #window2 $last, $($index,)*; );
    };
    {@gen } => {};
    {@item $first:literal; $($from:literal,)* ; $($to:literal,)*; $last:literal } => {
        paste::paste! {
            #[allow(clippy::unused_unit, unused_variables)]
            impl<
                [<T $first>],
                $([<T $to>],)*
                $([<M $from>]: Mapper<[<T $from>], Output = [<T $to>]>),*
            > Mapper<[<T $first>]> for ($([<M $from>],)*) {
                type Output = [<T $last>];
                fn map(value: T0) -> [<T $last>] {
                    $(
                        let value = [<M $from>]::map(value);
                    )*
                    value
                }
            }
        }
    };
    {$($tt:literal),* $(,)?} => {
        impl_mapper_for_tuple!(@gen #rev $($tt,)*;);
    };
}

impl_mapper_for_tuple! { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15 }

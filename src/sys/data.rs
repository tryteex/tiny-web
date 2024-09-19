use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::StrOrI64;

use super::{mail::MailProvider, response::Redirect, route::Route};

/// Data transferred between controllers, template, markers, database and cache
///
/// # Values
///
/// * `None` - No data transferred.
/// * `Usize(usize)` - No data transferred.
/// * `I16(i16)` - No data transferred.
/// * `I32(i32)` - No data transferred.
/// * `I64(i64)` - i64 data.
/// * `F32(f32)` - f32 data.
/// * `F64(f64)` - f64 data.
/// * `Bool(bool)` - bool data.
/// * `String(String)` - String data.
/// * `Date(DateTime<Utc>)` - Chrono dateTime.
/// * `Json(Value)` - Serde json.
/// * `Vec(Vec<Data>)` - List of `Data`.
/// * `Map(BTreeMap<i64, Data>)` - Map of `Data`.
/// * `Route(Route)` - Route data.
/// * `Redirect(Redirect)` - Redirect data.
/// * `MailProvider(MailProvider)` - Mail provider data.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Data {
    /// No data transferred.
    None,
    /// usize data.
    Usize(usize),
    /// i16 data.
    I16(i16),
    /// i32 data.
    I32(i32),
    /// i64 data.
    I64(i64),
    /// f32 data.
    F32(f32),
    /// f64 data.
    F64(f64),
    /// bool data.
    Bool(bool),
    /// String data.
    String(String),
    /// DateTime.
    Date(DateTime<Utc>),
    /// Json
    Json(Value),
    /// List of `Data`.
    Vec(Vec<Data>),
    /// Raw data,
    Raw(Vec<u8>),
    /// Map of `Data`.
    Map(BTreeMap<i64, Data>),
    /// Route data.
    #[serde(skip_serializing, skip_deserializing)]
    Route(Route),
    /// Redirect data.
    #[serde(skip_serializing, skip_deserializing)]
    Redirect(Redirect),
    /// Mail provider data
    #[serde(skip_serializing, skip_deserializing)]
    MailProvider(MailProvider),
}

macro_rules! impl_from_for_data {
    ($($type:ty => $variant:ident),* $(,)?) => {
        $(
            impl From<$type> for Data {
                fn from(value: $type) -> Self {
                    Data::$variant(value)
                }
            }

            impl From<Data> for $type {
                fn from(value: Data) -> Self {
                    if let Data::$variant(inner) = value {
                        inner
                    } else {
                        panic!("Cannot convert {:?} to {}", value, stringify!($type))
                    }
                }
            }

            impl<'a> From<&'a Data> for &'a $type {
                fn from(value: &'a Data) -> Self {
                    if let Data::$variant(inner) = value {
                        inner
                    } else {
                        panic!("Cannot convert {:?} to {}", value, stringify!($type))
                    }
                }
            }

        )*
    };
}

impl_from_for_data!(
    usize => Usize,
    i16 => I16,
    i32 => I32,
    i64 => I64,
    f32 => F32,
    f64 => F64,
    bool => Bool,
    String => String,
    DateTime<Utc> => Date,
    Value => Json,
    Vec<Data> => Vec,
    Vec<u8> => Raw,
    BTreeMap<i64, Data> => Map,
);

impl Data {
    pub fn get<T>(&self, key: impl StrOrI64) -> Option<&T>
    where
        for<'a> &'a T: From<&'a Data>,
    {
        match self {
            Data::Vec(vec) => match usize::try_from(key.to_i64()) {
                Ok(index) => {
                    let value = vec.get(index)?;
                    Some(value.into())
                }
                #[cfg(not(debug_assertions))]
                Err(_) => None,
                #[cfg(debug_assertions)]
                Err(e) => panic!("The key must be in the range for type usize. Err: {}", e),
            },
            Data::Map(map) => {
                let value = map.get(&key.to_i64())?;
                Some(value.into())
            }
            #[cfg(not(debug_assertions))]
            _ => None,
            #[cfg(debug_assertions)]
            _ => panic!("Must be Data::Map<Data> or Data::Vec<Data>"),
        }
    }

    pub fn take<T>(&mut self, key: impl StrOrI64) -> Option<T>
    where
        T: From<Data>,
    {
        match self {
            Data::Vec(vec) => match usize::try_from(key.to_i64()) {
                Ok(index) => {
                    let value = vec.remove(index);
                    Some(value.into())
                }
                #[cfg(not(debug_assertions))]
                Err(_) => None,
                #[cfg(debug_assertions)]
                Err(e) => panic!("The key must be in the range for type usize. Err: {}", e),
            },
            Data::Map(map) => {
                let value = map.remove(&key.to_i64())?;
                Some(value.into())
            }
            #[cfg(not(debug_assertions))]
            _ => None,
            #[cfg(debug_assertions)]
            _ => panic!("Must be Data::Map<Data> or Data::Vec<Data>"),
        }
    }
}

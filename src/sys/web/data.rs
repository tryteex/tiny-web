use std::collections::HashMap;

use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};

use serde_json::Value;

use crate::fnv1a_64;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Data {
    None,
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Usize(usize),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
    String(String),
    Date(DateTime<Utc>),
    Vec(Vec<Data>),
    Map(HashMap<i64, Data>),
    Raw(Vec<u8>),
    Json(Value),
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
    u8 => U8,
    u16 => U16,
    u32 => U32,
    u64 => U64,
    i8 => I8,
    i16 => I16,
    i32 => I32,
    i64 => I64,
    f32 => F32,
    f64 => F64,
    bool => Bool,
    String => String,
    DateTime<Utc> => Date,
    Vec<Data> => Vec,
    Vec<u8> => Raw,
    HashMap<i64, Data> => Map,
    Value => Json,
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
            _ => None,
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
            _ => None,
        }
    }
}

pub trait StrOrI64 {
    fn to_i64(&self) -> i64;
    fn to_str(&self) -> &str;
    fn is_str(&self) -> bool;
}

impl StrOrI64 for i64 {
    fn to_i64(&self) -> i64 {
        *self
    }
    fn to_str(&self) -> &str {
        ""
    }
    fn is_str(&self) -> bool {
        false
    }
}

impl StrOrI64 for &str {
    fn to_i64(&self) -> i64 {
        fnv1a_64(self.as_bytes())
    }
    fn to_str(&self) -> &str {
        self
    }
    fn is_str(&self) -> bool {
        true
    }
}

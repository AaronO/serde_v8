use rusty_v8 as v8;
use serde::de::{self, Visitor};
use serde::Deserialize;

use std::collections::HashMap;
use std::convert::TryFrom;

use crate::error::{Error, Result};
use crate::payload::ValueType;

pub struct Deserializer<'a, 'b, 's> {
    input: v8::Local<'a, v8::Value>,
    scope: &'b mut v8::HandleScope<'s>,
    _key_cache: Option<&'b mut KeyCache>,
}
pub type KeyCache = HashMap<&'static str, v8::Global<v8::String>>;

impl<'a, 'b, 's> Deserializer<'a, 'b, 's> {
    pub fn new(
        scope: &'b mut v8::HandleScope<'s>,
        input: v8::Local<'a, v8::Value>,
        key_cache: Option<&'b mut KeyCache>,
    ) -> Self {
        Deserializer {
            input,
            scope,
            _key_cache: key_cache,
        }
    }
}

// from_v8 deserializes a v8::Value into a Deserializable / rust struct
pub fn from_v8<'de, 'a, 'b, 's, T>(
    scope: &'b mut v8::HandleScope<'s>,
    input: v8::Local<'a, v8::Value>,
) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut deserializer = Deserializer::new(scope, input, None);
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

// like from_v8 except accepts a KeyCache to optimize struct key decoding
pub fn from_v8_cached<'de, 'a, 'b, 's, T>(
    scope: &'b mut v8::HandleScope<'s>,
    input: v8::Local<'a, v8::Value>,
    key_cache: &mut KeyCache,
) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut deserializer = Deserializer::new(scope, input, Some(key_cache));
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

macro_rules! wip {
    ($method:ident) => {
        fn $method<V>(self, _v: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            unimplemented!()
        }
    };
}

macro_rules! deserialize_signed {
    ($dmethod:ident, $vmethod:ident, $t:tt) => {
        fn $dmethod<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            visitor.$vmethod(self.input.integer_value(&mut self.scope).unwrap() as $t)
        }
    };
}

impl<'de, 'a, 'b, 's, 'x> de::Deserializer<'de> for &'x mut Deserializer<'a, 'b, 's> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match ValueType::from_v8(self.input) {
            ValueType::Null => self.deserialize_unit(visitor),
            ValueType::Bool => self.deserialize_bool(visitor),
            ValueType::Number => self.deserialize_f64(visitor),
            ValueType::String => self.deserialize_string(visitor),
            ValueType::Array => self.deserialize_seq(visitor),
            ValueType::Object => self.deserialize_map(visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.is_boolean() {
            visitor.visit_bool(self.input.boolean_value(&mut self.scope))
        } else {
            Err(Error::ExpectedBoolean)
        }
    }

    deserialize_signed!(deserialize_i8, visit_i8, i8);
    deserialize_signed!(deserialize_i16, visit_i16, i16);
    deserialize_signed!(deserialize_i32, visit_i32, i32);
    deserialize_signed!(deserialize_i64, visit_i64, i64);
    // TODO: maybe handle unsigned by itself ?
    deserialize_signed!(deserialize_u8, visit_u8, u8);
    deserialize_signed!(deserialize_u16, visit_u16, u16);
    deserialize_signed!(deserialize_u32, visit_u32, u32);
    deserialize_signed!(deserialize_u64, visit_u64, u64);

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.input.number_value(&mut self.scope).unwrap() as f32)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.input.number_value(&mut self.scope).unwrap())
    }

    wip!(deserialize_char);
    wip!(deserialize_str);

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.is_string() {
            let string = self
                .input
                .to_string(self.scope)
                .unwrap()
                .to_rust_string_lossy(self.scope);
            visitor.visit_string(string)
        } else {
            Err(Error::ExpectedString)
        }
    }

    wip!(deserialize_bytes);
    wip!(deserialize_byte_buf);

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.is_undefined() || self.input.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.is_null() {
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedNull)
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let arr = v8::Local::<v8::Array>::try_from(self.input).unwrap();
        let len = arr.length();
        let obj = v8::Local::<v8::Object>::from(arr);
        let seq = SeqAccess {
            pos: 0,
            len,
            obj,
            scope: self.scope,
        };
        visitor.visit_seq(seq)
    }

    // Like deserialize_seq except it prefers tuple's length over input array's length
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // TODO: error on length mismatch
        let obj = v8::Local::<v8::Object>::try_from(self.input).unwrap();
        let seq = SeqAccess {
            pos: 0,
            len: len as u32,
            obj,
            scope: self.scope,
        };
        visitor.visit_seq(seq)
    }

    // Tuple structs look just like sequences in JSON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    wip!(deserialize_map);

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let obj = v8::Local::<v8::Object>::try_from(self.input);
        let map = ObjectAccess {
            fields: fields,
            obj: obj.unwrap(),
            scope: self.scope,
            _cache: None,
        };

        visitor.visit_map(map)
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum. In JSON, struct fields and enum variants are
    // represented as strings. In other formats they may be represented as
    // numeric indices.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_none()
    }
}

struct ObjectAccess<'a, 'b, 's> {
    obj: v8::Local<'a, v8::Object>,
    scope: &'b mut v8::HandleScope<'s>,
    fields: &'static [&'static str],
    _cache: Option<&'b mut KeyCache>,
}

fn str_deserializer(s: &str) -> de::value::StrDeserializer<Error> {
    de::IntoDeserializer::into_deserializer(s)
}

// TODO: figure lifetimes out
// optimize rust -> v8 keys by using a KeyCache
// impl ObjectAccess<'_, '_, '_> {
//     fn get_key(&mut self, field: &'static str) -> v8::Local<'_, v8::Value> {
//         v8::String::new(self.scope, field).unwrap().into()
//     }

//     fn get_value(&mut self, field: &'static str) -> v8::Local<'_, v8::Value> {
//         let key = v8::String::new(self.scope, field).unwrap().into();
//         self.obj.get(self.scope, key).unwrap()
//     }
// }

impl<'de, 'a, 'b, 's> de::MapAccess<'de> for ObjectAccess<'a, 'b, 's> {
    type Error = Error;

    fn next_key_seed<K: de::DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        Ok(match self.fields.get(0) {
            Some(&field) => Some(seed.deserialize(str_deserializer(field))?),
            None => None,
        })
    }

    fn next_value_seed<V: de::DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let (field, fields) = self.fields.split_first().unwrap();
        self.fields = fields;
        let key = v8::String::new(self.scope, field).unwrap().into();
        let v8_val = self.obj.get(self.scope, key).unwrap();
        let mut deserializer = Deserializer::new(self.scope, v8_val, None);
        seed.deserialize(&mut deserializer)
    }

    fn next_entry_seed<K: de::DeserializeSeed<'de>, V: de::DeserializeSeed<'de>>(
        &mut self,
        kseed: K,
        vseed: V,
    ) -> Result<Option<(K::Value, V::Value)>> {
        Ok(match self.fields.split_first() {
            Some((&field, fields)) => {
                self.fields = fields;
                Some((kseed.deserialize(str_deserializer(field))?, {
                    let key = v8::String::new(self.scope, field).unwrap().into();
                    let v8_val = self.obj.get(self.scope, key).unwrap();
                    let mut deserializer = Deserializer::new(self.scope, v8_val, None);
                    vseed.deserialize(&mut deserializer)?
                }))
            }
            None => None,
        })
    }
}

struct SeqAccess<'a, 'b, 's> {
    obj: v8::Local<'a, v8::Object>,
    scope: &'b mut v8::HandleScope<'s>,
    len: u32,
    pos: u32,
}

impl<'de> de::SeqAccess<'de> for SeqAccess<'_, '_, '_> {
    type Error = Error;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>> {
        let pos = self.pos;
        self.pos += 1;

        if pos < self.len {
            let val = self.obj.get_index(self.scope, pos).unwrap();
            let mut deserializer = Deserializer::new(self.scope, val, None);
            Ok(Some(seed.deserialize(&mut deserializer)?))
        } else {
            Ok(None)
        }
    }
}

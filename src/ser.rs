use rusty_v8 as v8;
use serde::ser;
use serde::ser::{Impossible, Serialize};

use std::rc::Rc;
use std::cell::{RefCell};

use crate::error::{Error, Result};

type JsValue<'s> = v8::Local<'s, v8::Value>;
type JsResult<'s> = Result<JsValue<'s>>;

type ScopePtr<'s> = Rc<RefCell<v8::HandleScope<'s>>>;

pub fn to_v8<'a, 'b, T>(scope: &'a mut v8::HandleScope<'b>, input: T) -> JsResult<'a>
where
    T: Serialize,
{
    let subscope = v8::HandleScope::new(scope);
    let scopeptr = Rc::new(RefCell::new(subscope));
    let serializer = Serializer::new(scopeptr);
    let x = input.serialize(serializer);
    x
}

/// Wraps other serializers into an enum tagged variant form.
/// Uses {"Variant": ...payload...} for compatibility with serde-json.
pub struct VariantSerializer<'a, S> {
    variant: &'static str,
    inner: S,
    scope: ScopePtr<'a>,
}

impl<'a, S> VariantSerializer<'a, S> {
    pub fn new(scope: ScopePtr<'a>, variant: &'static str, inner: S) -> Self {
        Self { scope, variant, inner }
    }

    fn end(self, inner: impl FnOnce(S) -> JsResult<'a>) -> JsResult<'a> {
        let value = inner(self.inner)?;
        let scope = &mut *self.scope.borrow_mut();
        let obj = v8::Object::new(scope);
        let key = v8_struct_key(scope, self.variant).into();
        obj.set(scope, key, value);
        Ok(obj.into())
    }
}

impl<'a, S> ser::SerializeTupleVariant for VariantSerializer<'a, S>
where S: ser::SerializeTupleStruct<Ok = JsValue<'a>, Error = Error> 
{
    type Ok = JsValue<'a>;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.inner.serialize_field(value)
    }

    fn end(self) -> JsResult<'a> {
        self.end(S::end)
    }
}

impl<'a, S> ser::SerializeStructVariant for VariantSerializer<'a, S>
where S: ser::SerializeStruct<Ok = JsValue<'a>, Error = Error>
{
    type Ok = JsValue<'a>;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.inner.serialize_field(key, value)
    }

    fn end(self) -> JsResult<'a> {
        self.end(S::end)
    }
}

pub struct ArraySerializer<'a> {
    // serializer: Serializer<'a>,
    pending: Vec<JsValue<'a>>,
    scope: ScopePtr<'a>,
}

impl<'a> ArraySerializer<'a> {
    pub fn new(scope: ScopePtr<'a>) -> Self {
        // let serializer = Serializer::new(scope);
        Self {
            scope,
            // serializer,
            pending: vec![],
        }
    }
}

impl<'a> ser::SerializeSeq for ArraySerializer<'a> {
    type Ok = JsValue<'a>;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let x = value.serialize(Serializer::new(self.scope.clone()))?;
        self.pending.push(x);
        Ok(())
    }

    fn end(self) -> JsResult<'a> {
        let elements = self.pending.iter().as_slice();
        let scope = &mut *self.scope.borrow_mut();
        let arr = v8::Array::new_with_elements(scope, elements);
        Ok(arr.into())
    }
}

impl<'a> ser::SerializeTuple for ArraySerializer<'a> {
    type Ok = JsValue<'a>;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> JsResult<'a> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for ArraySerializer<'a> {
    type Ok = JsValue<'a>;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        ser::SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> JsResult<'a> {
        ser::SerializeTuple::end(self)
    }
}

pub struct ObjectSerializer<'a> {
    scope: ScopePtr<'a>,
    obj: v8::Local<'a, v8::Object>,
}

impl<'a> ObjectSerializer<'a> {
    pub fn new(scope: ScopePtr<'a>) -> Self {
        let obj = v8::Object::new(&mut *scope.borrow_mut());
        Self {
            scope,
            obj,
        }
    }
}

impl<'a> ser::SerializeStruct for ObjectSerializer<'a> {
    type Ok = JsValue<'a>;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        let value = value.serialize(Serializer::new(self.scope.clone()))?;
        let scope = &mut *self.scope.borrow_mut();
        let key = v8_struct_key(scope, key).into();
        self.obj.set(scope, key, value);
        Ok(())
    }

    fn end(self) -> JsResult<'a> {
        Ok(self.obj.into())
    }
}

#[derive(Clone)]
pub struct Serializer<'a> {
    scope: ScopePtr<'a>,
}

impl<'a> Serializer<'a> {
    pub fn new(scope: ScopePtr<'a>) -> Self {
        Serializer { scope }
    }
}

macro_rules! forward_to {
    ($($name:ident($ty:ty, $to:ident, $lt:lifetime);)*) => {
        $(fn $name(self, v: $ty) -> JsResult<$lt> {
            self.$to(v as _)
        })*
    };
}

impl<'a> ser::Serializer for Serializer<'a> {
    type Ok = v8::Local<'a, v8::Value>;
    type Error = Error;

    type SerializeSeq = ArraySerializer<'a>;
    type SerializeTuple = ArraySerializer<'a>;
    type SerializeTupleStruct = ArraySerializer<'a>;
    type SerializeTupleVariant = VariantSerializer<'a, ArraySerializer<'a>>;
    type SerializeMap = Impossible<v8::Local<'a, v8::Value>, Error>;
    type SerializeStruct = ObjectSerializer<'a>;
    type SerializeStructVariant = VariantSerializer<'a, ObjectSerializer<'a>>;

    forward_to! {
        serialize_i8(i8, serialize_i32, 'a);
        serialize_i16(i16, serialize_i32, 'a);

        serialize_u8(u8, serialize_u32, 'a);
        serialize_u16(u16, serialize_u32, 'a);

        serialize_f32(f32, serialize_f64, 'a);
        serialize_u64(u64, serialize_f64, 'a);
        serialize_i64(i64, serialize_f64, 'a);
    }
    
    fn serialize_i32(self, v: i32) -> JsResult<'a> {
        Ok(v8::Integer::new(&mut self.scope.borrow_mut(), v).into())
    }
    
    fn serialize_u32(self, v: u32) -> JsResult<'a> {
        Ok(v8::Integer::new_from_unsigned(&mut self.scope.borrow_mut(), v).into())
    }
    
    fn serialize_f64(self, v: f64) -> JsResult<'a> {
        Ok(v8::Number::new(&mut self.scope.borrow_mut(), v).into())
    }
    
    fn serialize_bool(self, v: bool) -> JsResult<'a> {
        Ok(v8::Boolean::new(&mut self.scope.borrow_mut(), v).into())
    }
    
    fn serialize_char(self, _v: char) -> JsResult<'a> {
        unimplemented!();
    }

    fn serialize_str(self, v: &str) -> JsResult<'a> {
        v8::String::new(&mut self.scope.borrow_mut(), v).map(|v| v.into()).ok_or(Error::ExpectedString)
    }

    fn serialize_bytes(self, _v: &[u8]) -> JsResult<'a> {
        // TODO: investigate using Uint8Arrays
        unimplemented!()
    }

    fn serialize_none(self) -> JsResult<'a> {
        Ok(v8::null(&mut self.scope.borrow_mut()).into())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> JsResult<'a> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> JsResult<'a> {
        Ok(v8::null(&mut self.scope.borrow_mut()).into())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> JsResult<'a> {
        Ok(v8::null(&mut self.scope.borrow_mut()).into())
    }

    /// For compatibility with serde-json, serialises unit variants as "Variant" strings.
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> JsResult<'a> {
        Ok(v8_struct_key(&mut self.scope.borrow_mut(), variant).into())
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> JsResult<'a> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> JsResult<'a> {
        let scope = self.scope.clone();
        let x = self.serialize_newtype_struct(variant, value)?;
        VariantSerializer::new(scope, variant, x).end(Ok)
    }

    /// Serialises any Rust iterable into a JS Array
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(ArraySerializer::new(self.scope))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(VariantSerializer::new(
            self.scope.clone(),
            variant,
            self.serialize_tuple_struct(variant, len)?,
        ))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        // TODO: serialize Maps (HashMap or BTreeMap) to v8 objects,
        // ideally JS Maps since they're lighter and better suited for K/V data
        // only allow certain keys (e.g: strings and numbers)
        unimplemented!()
    }

    /// Serialises Rust typed structs into plain JS objects.
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(ObjectSerializer::new(self.scope))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        let scope = self.scope.clone();
        let x = self.serialize_struct(variant, len)?;
        Ok(VariantSerializer::new(
            scope,
            variant,
            x,
        ))
    }
}

// creates an optimized v8::String for a struct field
// TODO: experiment with external strings
// TODO: evaluate if own KeyCache is better than v8's dedupe
fn v8_struct_key<'s>(scope: &mut v8::HandleScope<'s>, field: &'static str) -> v8::Local<'s, v8::String> {
    // Internalized v8 strings are significantly faster than "normal" v8 strings
    // since v8 deduplicates re-used strings minimizing new allocations
    // see: https://github.com/v8/v8/blob/14ac92e02cc3db38131a57e75e2392529f405f2f/include/v8.h#L3165-L3171
    v8::String::new_from_utf8(scope, field.as_ref(), v8::NewStringType::Internalized).unwrap().into()
}

use std::{
    error,
    fmt::{self, Display}
};

use serde::{ser::{self, Error, Impossible}, Serialize};

use winit::{
    event::{ElementState, MouseButton},
    keyboard::{Key, KeyCode, PhysicalKey}
};


#[derive(Debug)]
struct KError;

impl Display for KError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "expected key name")
    }
}

impl error::Error for KError {}

impl Error for KError
{
    fn custom<T: Display>(_: T) -> Self
    {
        KError
    }
}

macro_rules! unimpl_fn
{
    ($name:ident, $($arg:ty),*) =>
    {
        fn $name(self, $(_: $arg,)*) -> KResult
        {
            Err(KError)
        }
    }
}

type KResult = Result<&'static str, KError>;
type KImpossible = Impossible<&'static str, KError>;

struct KeyReader;

impl ser::Serializer for KeyReader
{
    type Ok = &'static str;
    type Error = KError;

    type SerializeSeq = KImpossible;
    type SerializeTuple = KImpossible;
    type SerializeTupleStruct = KImpossible;
    type SerializeTupleVariant = KImpossible;
    type SerializeMap = KImpossible;
    type SerializeStruct = KImpossible;
    type SerializeStructVariant = KImpossible;


    unimpl_fn!{serialize_bool, bool}

    unimpl_fn!{serialize_i8, i8}
    unimpl_fn!{serialize_i16, i16}
    unimpl_fn!{serialize_i32, i32}
    unimpl_fn!{serialize_i64, i64}

    unimpl_fn!{serialize_u8, u8}
    unimpl_fn!{serialize_u16, u16}
    unimpl_fn!{serialize_u32, u32}
    unimpl_fn!{serialize_u64, u64}

    unimpl_fn!{serialize_f32, f32}
    unimpl_fn!{serialize_f64, f64}

    unimpl_fn!{serialize_char, char}

    unimpl_fn!{serialize_str, &str}
    unimpl_fn!{serialize_bytes, &[u8]}

    unimpl_fn!{serialize_none,}
    unimpl_fn!{serialize_unit,}

    unimpl_fn!{serialize_unit_struct, &'static str}

    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> KResult
    {
        Err(KError)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str
    ) -> KResult
    {
        Ok(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: &T
    ) -> KResult
    {
        Err(KError)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T
    ) -> KResult
    {
        Err(KError)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, KError>
    {
        Err(KError)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, KError>
    {
        Err(KError)
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize
    ) -> Result<Self::SerializeTupleStruct, KError>
    {
        Err(KError)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize
    ) -> Result<Self::SerializeTupleVariant, KError>
    {
        Err(KError)
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, KError>
    {
        Err(KError)
    }

    fn serialize_struct(
        self,
        _: &'static str,
        _: usize
    ) -> Result<Self::SerializeStruct, KError>
    {
        Err(KError)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize
    ) -> Result<Self::SerializeStructVariant, KError>
    {
        Err(KError)
    }
}

pub struct KeyCodeNamed(pub KeyCode);

impl Display for KeyCodeNamed
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "{}", self.0.serialize(KeyReader).unwrap())
    }
}

#[derive(Debug, Clone)]
pub enum Control
{
    Keyboard{logical: Key, keycode: PhysicalKey, state: ElementState},
    Mouse{button: MouseButton, state: ElementState},
    Scroll{x: f64, y: f64}
}

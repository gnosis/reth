// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use rlp::{Rlp, RlpIterator};
use serde::de::{DeserializeSeed, SeqAccess};
use serde_rlp::error::ErrorKind;

use crate as serde_rlp;

#[derive(Debug)]
pub(crate) struct EthereumRlpDeserializer<'de> {
    input: &'de [u8],
    current: Rlp<'de>,
}

impl<'de> EthereumRlpDeserializer<'de> {
    pub fn from_slice(input: &'de [u8]) -> Self {
        EthereumRlpDeserializer {
            input: input,
            current: Rlp::new(&input),
        }
    }
}

impl<'de, 'a> serde::Deserializer<'de> for &'a mut EthereumRlpDeserializer<'de> {
    type Error = serde_rlp::error::ErrorKind;

    fn deserialize_any<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(ErrorKind::RlpAnyNotSupported)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_bool(self.current.as_val().unwrap())
    }

    fn deserialize_i8<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(ErrorKind::RlpSignedIntegersNotSupported)
    }

    fn deserialize_i16<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(ErrorKind::RlpSignedIntegersNotSupported)
    }

    fn deserialize_i32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(ErrorKind::RlpSignedIntegersNotSupported)
    }

    fn deserialize_i64<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(ErrorKind::RlpSignedIntegersNotSupported)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u8(self.current.as_val().unwrap())
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u16(self.current.as_val().unwrap())
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u32(self.current.as_val().unwrap())
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u64(self.current.as_val().unwrap())
    }

    fn deserialize_f32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(ErrorKind::RlpFloatingPorintNotSupported)
    }

    fn deserialize_f64<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(ErrorKind::RlpFloatingPorintNotSupported)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_char(self.current.as_val::<u8>().unwrap() as char)
    }

    fn deserialize_str<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(ErrorKind::RlpIntoBorrowedStringDeserializationNotSupported)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
      visitor.visit_string(self.current.as_val::<String>().unwrap())
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        struct Access<'a, 'de> {
            de: &'a mut EthereumRlpDeserializer<'de>,
            it: &'a mut RlpIterator<'de, 'a>,
        }

        impl<'a, 'de> SeqAccess<'de> for Access<'a, 'de> {
            type Error = serde_rlp::error::ErrorKind;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                if let Some(v) = self.it.next() {
                    self.de.current = v;
                    let value = DeserializeSeed::deserialize(seed, &mut *self.de)?;
                    Ok(Some(value))
                } else {
                    Ok(None)
                }
            }
        }

        let rlp = Rlp::new(&self.input);
        visitor.visit_seq(Access {
            de: self,
            it: &mut rlp.into_iter(),
        })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }
}

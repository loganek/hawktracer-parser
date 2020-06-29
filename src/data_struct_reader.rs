use crate::data_provider::{DataError, DataProvider};
use crate::event::{DataType, Event, Value};
use crate::event_klass::{EventKlass, EventKlassField};
use crate::registry::EventKlassRegistry;

#[derive(Debug, PartialEq)]
pub enum ReadEventError {
    DataError(DataError),
    UnknownKlass(String),
    UnknownKlassId(u32),
    RegistryUpdateFailed(String),
}

pub struct DataStructReader<'a> {
    data_provider: &'a mut DataProvider,
    registry: &'a EventKlassRegistry,
    base_event: Option<Event>,
    klass: &'a EventKlass,
}

macro_rules! get_integer {
    ($self: ident, $type: ty, $size: expr, $data_type: ident) => {{
        let mut buffer: [u8; $size] = [0; $size];
        match $self.data_provider.read_bytes(&mut buffer) {
            Ok(()) => unsafe {
                Ok(Value::$data_type(
                    std::mem::transmute::<[u8; $size], $type>(buffer),
                ))
            },
            Err(err) => Err(ReadEventError::DataError(err)),
        }
    }};
}

impl<'a> DataStructReader<'a> {
    pub fn new(
        data_provider: &'a mut DataProvider,
        registry: &'a EventKlassRegistry,
        klass: &'a EventKlass,
        base_event: Option<Event>,
    ) -> DataStructReader<'a> {
        DataStructReader {
            data_provider,
            registry,
            base_event,
            klass,
        }
    }

    pub fn read_event(&mut self) -> Result<Event, ReadEventError> {
        self.read_event_internal(self.klass)
    }

    fn read_event_internal(&mut self, klass: &EventKlass) -> Result<Event, ReadEventError> {
        let mut values = std::collections::HashMap::<String, Value, fnv::FnvBuildHasher>::default();
        for field in klass.get_fields() {
            values.insert(field.get_name().clone(), self.read_field(&field)?);
        }

        Ok(Event::new(klass.get_id(), values))
    }

    fn read_field(&mut self, field: &EventKlassField) -> Result<Value, ReadEventError> {
        match field.get_data_type() {
            DataType::U8 => get_integer!(self, u8, 1, U8),
            DataType::I8 => get_integer!(self, i8, 1, I8),
            DataType::U16 => get_integer!(self, u16, 2, U16),
            DataType::I16 => get_integer!(self, i16, 2, I16),
            DataType::U32 => get_integer!(self, u32, 4, U32),
            DataType::I32 => get_integer!(self, i32, 4, I32),
            DataType::U64 => get_integer!(self, u64, 8, U64),
            DataType::I64 => get_integer!(self, i64, 8, I64),
            DataType::Str => self.read_string(),
            DataType::Struct => self.read_struct(field),
        }
    }

    fn read_struct(&mut self, field: &EventKlassField) -> Result<Value, ReadEventError> {
        if field.get_type_name() == "HT_Event" && field.get_name() == "base" {
            let base_event = std::mem::replace(&mut self.base_event, None);
            Ok(Value::Struct(base_event.expect(
                "Base event must be provided for non-base events.",
            )))
        } else if let Some(klass) = self.registry.get_klass_by_name(field.get_type_name()) {
            match self.read_event_internal(klass) {
                Ok(value) => Ok(Value::Struct(value)),
                Err(err) => Err(err),
            }
        } else {
            Err(ReadEventError::UnknownKlass(field.get_type_name().clone()))
        }
    }

    fn read_string(&mut self) -> Result<Value, ReadEventError> {
        match self.data_provider.read_string() {
            Ok(data) => Ok(Value::Str(data)),
            Err(err) => Err(ReadEventError::DataError(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hawktracer_parser_test_utilities::FakeDataReader;

    fn value_from_bytes(buff: Vec<u8>, data_type: DataType) -> Value {
        let mut data_provider = DataProvider::new(Box::new(FakeDataReader::new(buff, false)));
        let klass = EventKlass::new(99, "foo".to_owned());
        DataStructReader::new(&mut data_provider, &EventKlassRegistry::new(), &klass, None)
            .read_field(&EventKlassField::new(
                "foo".to_owned(),
                "bar".to_owned(),
                data_type,
            ))
            .unwrap()
    }

    #[test]
    fn read_field_value() {
        assert_eq!(value_from_bytes(vec![255], DataType::I8), Value::I8(-1));
        assert_eq!(value_from_bytes(vec![52], DataType::I8), Value::I8(52));
        assert_eq!(value_from_bytes(vec![240], DataType::U8), Value::U8(240));

        assert_eq!(value_from_bytes(vec![5, 1], DataType::I16), Value::I16(261));
        assert_eq!(
            value_from_bytes(vec![50, 200], DataType::I16),
            Value::I16(-14286)
        );
        assert_eq!(
            value_from_bytes(vec![240, 52], DataType::U16),
            Value::U16(13552)
        );

        assert_eq!(
            value_from_bytes(vec![10, 16, 42, 169], DataType::I32),
            Value::I32(-1456861174)
        );
        assert_eq!(
            value_from_bytes(vec![255, 255, 255, 127], DataType::I32),
            Value::I32(2147483647)
        );
        assert_eq!(
            value_from_bytes(vec![140, 23, 50, 190], DataType::U32),
            Value::U32(3190953868)
        );

        assert_eq!(
            value_from_bytes(vec![253, 255, 255, 255, 255, 255, 255, 255], DataType::I64),
            Value::I64(-3)
        );
        assert_eq!(
            value_from_bytes(vec![2, 0, 0, 0, 0, 0, 0, 29], DataType::I64),
            Value::I64(2089670227099910146)
        );
        assert_eq!(
            value_from_bytes(vec![1, 2, 3, 4, 5, 6, 7, 8], DataType::U64),
            Value::U64(578437695752307201)
        );

        assert_eq!(
            value_from_bytes(vec![65, 66, 67, 0], DataType::Str),
            Value::Str("ABC".to_owned())
        );
    }

    #[test]
    fn data_struct_reader_should_convert_valid_byte_stream_to_event() {
        let mut child_klass = EventKlass::new(99, "ChildKlass".to_owned());
        child_klass.add_field("i8_field".to_owned(), "int8_t".to_owned(), DataType::I8);

        let mut klass = EventKlass::new(100, "foo".to_owned());
        klass.add_field(
            "child_klass".to_owned(),
            "ChildKlass".to_owned(),
            DataType::Struct,
        );
        klass.add_field("str_field".to_owned(), "char*".to_owned(), DataType::Str);
        klass.add_field("u32_field".to_owned(), "uint32_t".to_owned(), DataType::U32);

        let data = vec![
            128, // -128
            65, 66, 67, 0, // ABC
            45, 1, 0, 0, // 301
        ];

        let mut reg = EventKlassRegistry::new();
        reg.add_klass(child_klass);

        let mut data_provider = DataProvider::new(Box::new(FakeDataReader::new(data, false)));
        let mut reader = DataStructReader::new(&mut data_provider, &reg, &klass, None);

        let res = reader.read_event().unwrap();
        assert_eq!(res.get_klass_id(), 100);
        match res.get_raw_value(&"child_klass").unwrap() {
            Value::Struct(event) => {
                assert_eq!(event.get_raw_value(&"i8_field").unwrap(), &Value::I8(-128))
            }
            _ => assert!(false),
        };
        assert_eq!(
            res.get_raw_value(&"str_field").unwrap(),
            &Value::Str("ABC".to_owned())
        );
        assert_eq!(res.get_raw_value(&"u32_field").unwrap(), &Value::U32(301));
    }

    #[test]
    fn reader_should_fail_for_invalid_klass() {
        let mut klass = EventKlass::new(100, "foo".to_owned());
        klass.add_field(
            "child_klass".to_owned(),
            "UnknownKlass".to_owned(),
            DataType::Struct,
        );

        let data = vec![
            128, // -128
            65, 66, 67, 0, // ABC
            45, 1, 0, 0, // 301
        ];

        let reg = EventKlassRegistry::new();

        let mut data_provider = DataProvider::new(Box::new(FakeDataReader::new(data, false)));
        let mut reader = DataStructReader::new(&mut data_provider, &reg, &klass, None);

        assert_eq!(
            ReadEventError::UnknownKlass("UnknownKlass".to_owned()),
            reader.read_event().unwrap_err()
        );
    }

    #[test]
    fn reader_should_fail_for_invalid_string() {
        let data = vec![65, 66, 67];

        let mut data_provider = DataProvider::new(Box::new(FakeDataReader::new(data, false)));
        let err = DataStructReader::new(
            &mut data_provider,
            &EventKlassRegistry::new(),
            &EventKlass::new(100, "foo".to_owned()),
            None,
        )
        .read_string()
        .unwrap_err();

        assert_eq!(ReadEventError::DataError(DataError::EndOfStream), err);
    }
}

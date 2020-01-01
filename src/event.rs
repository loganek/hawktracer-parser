#[derive(Copy, Clone, PartialEq, Debug)]
pub enum DataType {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    Str,
    Struct,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ErrorKind {
    NotFound,
    InvalidType,
}

#[derive(Debug, PartialEq)]
pub struct Event {
    klass_id: u32,
    values: std::collections::HashMap<String, Value>,
}

#[derive(Debug)]
pub struct ValueError {
    kind: ErrorKind,
    field: String,
}

impl ValueError {
    pub fn new(field: &str, kind: ErrorKind) -> ValueError {
        ValueError {
            kind,
            field: field.to_string(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind.clone()
    }
}

impl std::error::Error for ValueError {}

impl std::fmt::Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Cannot get value {}: {:?}>", self.field, self.kind)
    }
}

// Keep in sync with DataType
// TODO: can we merge those two enums?
#[derive(Debug, PartialEq)]
pub enum Value {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    Str(String),
    Struct(Event),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::U8(v) => write!(f, "{}", v),
            Value::I8(v) => write!(f, "{}", v),
            Value::U16(v) => write!(f, "{}", v),
            Value::I16(v) => write!(f, "{}", v),
            Value::U32(v) => write!(f, "{}", v),
            Value::I32(v) => write!(f, "{}", v),
            Value::U64(v) => write!(f, "{}", v),
            Value::I64(v) => write!(f, "{}", v),
            Value::Str(v) => write!(f, "\"{}\"", v),
            Value::Struct(v) => write!(f, "<Event {}>", v.get_klass_id()),
        }
    }
}

macro_rules! make_field_getter {
    ($function_name: ident, $data_type: ident, $type: ty) => (
        pub fn $function_name(&self, name: &str) -> Result<$type, ValueError> {
            match self.values.get(name) {
                Some(value) => {
                    if let Value::$data_type(data) = value {
                        Ok(*data)
                    } else {
                        Err(ValueError::new(name, ErrorKind::InvalidType))
                    }
                },
                None => Err(ValueError::new(name, ErrorKind::NotFound))
            }
        }
    )
}

macro_rules! make_field_getter_ref {
    ($function_name: ident, $data_type: ident, $type: ty) => (
        pub fn $function_name(&self, name: &str) -> Result<$type, ValueError> {
            match self.values.get(name) {
                Some(value) => {
                    if let Value::$data_type(data) = value {
                        Ok(data)
                    } else {
                        Err(ValueError::new(name, ErrorKind::InvalidType))
                    }
                },
                None => Err(ValueError::new(name, ErrorKind::NotFound))
            }
        }
    )
}

impl Event {
    pub fn new(klass_id: u32, values: std::collections::HashMap<String, Value>) -> Event {
        Event { klass_id, values }
    }

    make_field_getter!(get_value_u8, U8, u8);
    make_field_getter!(get_value_i8, I8, i8);
    make_field_getter!(get_value_u16, U16, u16);
    make_field_getter!(get_value_i16, I16, i16);
    make_field_getter!(get_value_u32, U32, u32);
    make_field_getter!(get_value_i32, I32, i32);
    make_field_getter!(get_value_u64, U64, u64);
    make_field_getter!(get_value_i64, I64, i64);
    make_field_getter_ref!(get_value_string, Str, &String);
    make_field_getter_ref!(get_value_struct, Struct, &Event);

    pub fn get_raw_value(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }

    pub fn get_all_values(&self) -> &std::collections::HashMap<String, Value> {
        &self.values
    }

    pub fn get_klass_id(&self) -> u32 {
        self.klass_id
    }

    pub fn flat_event(self) -> Event {
        let mut new_values = std::collections::HashMap::<String, Value>::new();
        let klass_id = self.get_klass_id();
        self.flat_event_internal(&mut new_values);

        Event::new(klass_id, new_values)
    }

    fn flat_event_internal(mut self, new_values: &mut std::collections::HashMap<String, Value>) {
        let base_value = self.values.remove("base");

        for (name, value) in self.values {
            new_values.insert(name, value);
        }

        if let Some(base_value) = base_value {
            if let Value::Struct(event) = base_value {
                event.flat_event_internal(new_values);
            } else {
                new_values.insert("base".to_string(), base_value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn getting_klass_id_should_return_correct_value() {
        let klass_id = 5;
        let event = Event::new(klass_id, HashMap::<String, Value>::new());
        assert_eq!(klass_id, event.get_klass_id());
    }

    #[test]
    fn getting_valid_type_should_not_fail() {
        let u32_value = 492;
        let mut values = HashMap::<String, Value>::new();
        values.insert("v1".to_string(), Value::U32(u32_value));
        let event = Event::new(1, values);

        assert_eq!(
            event.get_value_u32("v1").expect("value v1 doesn't exist"),
            u32_value
        );
    }

    #[test]
    fn getting_non_existing_value_should_fail() {
        let event = Event::new(1, HashMap::<String, Value>::new());

        assert_eq!(
            event.get_value_u32("non-existing").unwrap_err().kind(),
            ErrorKind::NotFound
        );
    }

    #[test]
    fn getting_non_existing_string_value_should_fail() {
        let event = Event::new(1, HashMap::<String, Value>::new());

        assert_eq!(
            event.get_value_string("non-existing").unwrap_err().kind(),
            ErrorKind::NotFound
        );
    }

    #[test]
    fn getting_invalid_type_should_fail() {
        let mut values = HashMap::<String, Value>::new();
        values.insert("v1".to_string(), Value::U32(2));
        let event = Event::new(1, values);

        assert_eq!(
            event.get_value_string("v1").unwrap_err().kind(),
            ErrorKind::InvalidType
        );
    }

    #[test]
    fn getting_invalid_integer_type_should_fail() {
        let mut values = HashMap::<String, Value>::new();
        values.insert("v1".to_string(), Value::U8(2));
        let event = Event::new(1, values);

        assert_eq!(
            event.get_value_u32("v1").unwrap_err().kind(),
            ErrorKind::InvalidType
        );
    }

    #[test]
    fn flatten_event_should_collapse_all_base_struct_events() {
        let mut super_base_values = HashMap::<String, Value>::new();
        super_base_values.insert("timestamp".to_string(), Value::U64(999));
        super_base_values.insert("xxx".to_string(), Value::U64(876));

        let mut base_values = HashMap::<String, Value>::new();
        base_values.insert(
            "base".to_string(),
            Value::Struct(Event::new(1, super_base_values)),
        );
        base_values.insert("timestamp".to_string(), Value::U64(123));
        base_values.insert("id".to_string(), Value::U64(456));

        let mut values = HashMap::<String, Value>::new();
        values.insert(
            "base".to_string(),
            Value::Struct(Event::new(1, base_values)),
        );
        values.insert("name".to_string(), Value::Str("some_name".to_string()));
        let event = Event::new(3, values);

        let event = event.flat_event();

        assert_eq!(4, event.values.len());
        assert_eq!(3, event.get_klass_id());
        assert_eq!(event.get_value_u64("timestamp").unwrap(), 999);
        assert_eq!(event.get_value_u64("id").unwrap(), 456);
        assert_eq!(event.get_value_u64("xxx").unwrap(), 876);
        assert_eq!(event.get_value_string("name").unwrap(), "some_name");
    }

    #[test]
    fn flatten_event_should_not_collapse_non_event_fields() {
        let mut values = HashMap::<String, Value>::new();
        values.insert("base".to_string(), Value::U64(2));
        values.insert("name".to_string(), Value::Str("some_name".to_string()));
        let event = Event::new(3, values);

        let event = event.flat_event();

        assert_eq!(2, event.values.len());
        assert_eq!(event.get_value_u64("base").unwrap(), 2);
        assert_eq!(event.get_value_string("name").unwrap(), "some_name");
    }
}

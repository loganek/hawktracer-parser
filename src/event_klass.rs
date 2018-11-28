use crate::event::DataType;

pub struct EventKlassField {
    name: String,
    type_name: String,
    data_type: DataType,
}

pub struct EventKlass {
    fields: std::vec::Vec<EventKlassField>,
    name: String,
    id: u32,
}

impl EventKlass {
    pub fn new(id: u32, name: String) -> EventKlass {
        EventKlass {
            fields: vec![],
            name,
            id,
        }
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_fields(&self) -> &std::vec::Vec<EventKlassField> {
        &self.fields
    }

    pub fn add_field(&mut self, name: String, type_name: String, data_type: DataType) {
        for field in &self.fields {
            if *field.get_name() == name {
                return; // TODO error?
            }
        }
        self.fields
            .push(EventKlassField::new(name, type_name, data_type));
    }
}

impl EventKlassField {
    pub fn new(name: String, type_name: String, data_type: DataType) -> EventKlassField {
        EventKlassField {
            name,
            type_name,
            data_type,
        }
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_data_type(&self) -> &DataType {
        &self.data_type
    }

    pub fn get_type_name(&self) -> &String {
        &self.type_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_klass_name_should_return_correct_value() {
        assert_eq!(
            *EventKlass::new(9, "klass_name".to_string()).get_name(),
            "klass_name".to_string()
        );
    }

    #[test]
    fn get_klass_name_id_return_correct_value() {
        assert_eq!(EventKlass::new(9, "klass_name".to_string()).get_id(), 9);
    }

    #[test]
    fn insert_field_should_add_new_field_to_klass() {
        let mut klass = EventKlass::new(9, "klass_name".to_string());
        klass.add_field("name".to_string(), "type".to_string(), DataType::U32);

        let field = &klass.get_fields()[0];
        assert_eq!(*field.get_data_type(), DataType::U32);
        assert_eq!(*field.get_type_name(), "type".to_string());
        assert_eq!(*field.get_name(), "name".to_string());

        assert_eq!(klass.get_fields().len(), 1);
    }

    #[test]
    fn insert_field_with_the_same_name_twice_should_only_add_first_field() {
        let mut klass = EventKlass::new(9, "klass_name".to_string());
        klass.add_field("name".to_string(), "type1".to_string(), DataType::U32);
        klass.add_field("name".to_string(), "type2".to_string(), DataType::U8);

        let field = &klass.get_fields()[0];
        assert_eq!(*field.get_data_type(), DataType::U32);
        assert_eq!(*field.get_type_name(), "type1".to_string());
        assert_eq!(*field.get_name(), "name".to_string());

        assert_eq!(klass.get_fields().len(), 1);
    }
}

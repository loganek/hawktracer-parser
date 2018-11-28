use crate::event::DataType;
use crate::event::Event;
use crate::event_klass::EventKlass;
use crate::registry::CoreEventKlassId;
use crate::registry::EventKlassRegistry;

pub struct RegistryUpdater<'a> {
    registry: &'a mut EventKlassRegistry,
}

impl<'a> RegistryUpdater<'a> {
    pub fn new(registry: &'a mut EventKlassRegistry) -> RegistryUpdater<'a> {
        RegistryUpdater { registry }
    }

    pub fn update_registry_from_event(&mut self, event: &Event) -> Result<(), &'static str> {
        match event.get_klass_id() {
            x if x == CoreEventKlassId::KlassInfo as u32 => self.add_new_klass(&event),
            x if x == CoreEventKlassId::FieldInfo as u32 => self.add_klass_field(&event),
            _ => Err("Klass id is neither KlassInfo nor FieldInfo"),
        }
    }

    fn add_new_klass(&mut self, event: &Event) -> Result<(), &'static str> {
        let klass_id = match event.get_value_u32("info_klass_id") {
            Ok(value) => value,
            Err(_) => return Err("Cannot read klass id"),
        };

        if CoreEventKlassId::is_core_klass(klass_id) {
            return Ok(());
        }

        let klass_name = match event.get_value_string("event_klass_name") {
            Ok(value) => value.clone(),
            Err(_) => return Err("Cannot read klass name"),
        };

        self.registry
            .add_klass(EventKlass::new(klass_id, klass_name));
        Ok(())
    }

    fn add_klass_field(&mut self, event: &Event) -> Result<(), &'static str> {
        let klass_id = match event.get_value_u32("info_klass_id") {
            Ok(value) => value,
            Err(_) => return Err("Cannot read klass id"),
        };

        if CoreEventKlassId::is_core_klass(klass_id) {
            return Ok(()); // Ignore core fields
        }

        let field_name = match event.get_value_string("field_name") {
            Ok(value) => value.clone(),
            Err(_) => return Err("Cannot read field name"),
        };

        let type_name = match event.get_value_string("field_type") {
            Ok(value) => value.clone(),
            Err(_) => return Err("Cannot read field type"),
        };

        let data_type = match event.get_value_u8("data_type") {
            Ok(value) => match value {
                1 => DataType::Struct,
                2 => DataType::Str,
                6 => DataType::U64, // TODO it's a pointer!
                99 => {
                    if let Ok(size) = event.get_value_u64("size") {
                        match size {
                            1 => DataType::U8,
                            4 => DataType::U32,
                            8 => DataType::U64,
                            _ => return Err("Invalid size of integer type"),
                        }
                    } else {
                        return Err("Cannot read field size");
                    }
                }
                _ => return Err("Invalid data type"),
            },
            Err(_) => return Err("Cannot read field data type"),
        };

        match self.registry.get_klass_by_id_mut(klass_id) {
            Some(klass) => {
                klass.add_field(field_name, type_name, data_type);
                Ok(())
            }
            None => Err("Cannot find klass"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Value;

    fn make_klass_info_event(
        id: Option<u32>,
        name: Option<&str>,
        field_count: Option<u8>,
    ) -> Event {
        let mut values = std::collections::HashMap::new();

        if id.is_some() {
            values.insert("info_klass_id".to_string(), Value::U32(id.unwrap()));
        }
        if name.is_some() {
            values.insert(
                "event_klass_name".to_string(),
                Value::Str(name.unwrap().to_string()),
            );
        }
        if field_count.is_some() {
            values.insert("field_count".to_string(), Value::U8(field_count.unwrap()));
        }

        Event::new(CoreEventKlassId::KlassInfo as u32, values)
    }

    fn make_field_info_event(
        klass_id: Option<u32>,
        field_type: Option<&str>,
        field_name: Option<&str>,
        size: Option<u64>,
        data_type: Option<u8>,
    ) -> Event {
        let mut values = std::collections::HashMap::new();

        if klass_id.is_some() {
            values.insert("info_klass_id".to_string(), Value::U32(klass_id.unwrap()));
        }
        if field_type.is_some() {
            values.insert(
                "field_type".to_string(),
                Value::Str(field_type.unwrap().to_string()),
            );
        }
        if field_name.is_some() {
            values.insert(
                "field_name".to_string(),
                Value::Str(field_name.unwrap().to_string()),
            );
        }
        if size.is_some() {
            values.insert("size".to_string(), Value::U64(size.unwrap()));
        }
        if data_type.is_some() {
            values.insert("data_type".to_string(), Value::U8(data_type.unwrap()));
        }

        Event::new(CoreEventKlassId::FieldInfo as u32, values)
    }

    #[test]
    fn should_fail_if_event_is_not_field_or_klass_info_event() {
        let mut registry = EventKlassRegistry::new();
        let mut updater = RegistryUpdater::new(&mut registry);
        let event = Event::new(99, std::collections::HashMap::new());

        assert!(updater.update_registry_from_event(&event).is_err());
    }

    #[test]
    fn should_add_new_klass_to_registry_if_all_fields_are_in_event() {
        let mut registry = EventKlassRegistry::new();

        {
            let mut updater = RegistryUpdater::new(&mut registry);
            assert!(updater
                .update_registry_from_event(&make_klass_info_event(Some(99), Some("name"), Some(0)))
                .is_ok());
        }

        assert!(registry.get_klass_by_id(99).is_some());
        assert!(registry.get_klass_by_name("name").is_some());
    }

    #[test]
    fn add_new_klass_to_registry_if_some_fields_are_missing_should_fail() {
        let mut registry = EventKlassRegistry::new();

        {
            let mut updater = RegistryUpdater::new(&mut registry);
            assert!(updater
                .update_registry_from_event(&make_klass_info_event(None, Some("name"), Some(0)))
                .is_err());
            assert!(updater
                .update_registry_from_event(&make_klass_info_event(Some(99), None, Some(0)))
                .is_err());
        }
    }

    #[test]
    fn add_core_klass_to_registry_should_not_fail() {
        let mut registry = EventKlassRegistry::new();

        {
            let mut updater = RegistryUpdater::new(&mut registry);
            assert!(updater
                .update_registry_from_event(&make_klass_info_event(
                    Some(CoreEventKlassId::Base as u32),
                    Some("new_name"),
                    Some(0)
                ))
                .is_ok());
        }

        assert_eq!(
            *registry
                .get_klass_by_id(CoreEventKlassId::Base as u32)
                .unwrap()
                .get_name(),
            "HT_Event".to_string()
        );
    }

    #[test]
    fn add_field_to_non_existing_klass_should_fail() {
        let mut registry = EventKlassRegistry::new();

        {
            let mut updater = RegistryUpdater::new(&mut registry);
            let event = make_field_info_event(Some(99), Some("t"), Some("n"), Some(4), Some(99));
            assert!(updater.update_registry_from_event(&event).is_err());
        }
    }

    #[test]
    fn add_field_if_some_values_are_missing_should_fail() {
        let mut registry = EventKlassRegistry::new();
        let mut updater = RegistryUpdater::new(&mut registry);
        assert!(updater
            .update_registry_from_event(&make_klass_info_event(Some(99), Some("name"), Some(10)))
            .is_ok());

        assert!(updater
            .update_registry_from_event(&make_field_info_event(
                None,
                Some("t"),
                Some("n"),
                Some(4),
                Some(1)
            ))
            .is_err());
        assert!(updater
            .update_registry_from_event(&make_field_info_event(
                Some(99),
                None,
                Some("n"),
                Some(4),
                Some(2)
            ))
            .is_err());
        assert!(updater
            .update_registry_from_event(&make_field_info_event(
                Some(99),
                Some("u"),
                None,
                Some(4),
                Some(6)
            ))
            .is_err());
        assert!(updater
            .update_registry_from_event(&make_field_info_event(
                Some(99),
                Some("v"),
                Some("n"),
                None,
                Some(99)
            ))
            .is_err());
        assert!(updater
            .update_registry_from_event(&make_field_info_event(
                Some(99),
                Some("w"),
                Some("n"),
                Some(99),
                None
            ))
            .is_err());
    }

    #[test]
    fn add_field_to_core_klass_should_not_update_klass() {
        let mut registry = EventKlassRegistry::new();

        {
            let mut updater = RegistryUpdater::new(&mut registry);
            let event = make_field_info_event(
                Some(CoreEventKlassId::Base as u32),
                Some("t"),
                Some("n"),
                Some(4),
                Some(99),
            );
            assert!(updater.update_registry_from_event(&event).is_ok());
        }

        assert_eq!(
            registry
                .get_klass_by_id(CoreEventKlassId::Base as u32)
                .unwrap()
                .get_fields()
                .len(),
            3
        );
    }

    #[test]
    fn add_uint_field_with_invalid_size_should_fail() {
        let mut registry = EventKlassRegistry::new();
        let mut updater = RegistryUpdater::new(&mut registry);
        assert!(updater
            .update_registry_from_event(&make_klass_info_event(Some(99), Some("name"), Some(10)))
            .is_ok());

        assert!(updater
            .update_registry_from_event(&make_field_info_event(
                Some(99),
                Some("t"),
                Some("n"),
                Some(99),
                Some(10)
            ))
            .is_err());
    }
}

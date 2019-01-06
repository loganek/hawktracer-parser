use crate::event::DataType;
use crate::event_klass::EventKlass;

#[derive(Copy, Clone)]
pub enum CoreEventKlassId {
    Endianness = 0,
    Base = 1,
    KlassInfo = 2,
    FieldInfo = 3,
}

impl CoreEventKlassId {
    pub fn is_core_klass(klass_id: u32) -> bool {
        use self::CoreEventKlassId::*;
        for val in &[Base, Endianness, FieldInfo, KlassInfo] {
            if *val as u32 == klass_id {
                return true;
            }
        }
        false
    }
}

#[derive(Default)]
pub struct EventKlassRegistry {
    klasses: std::collections::HashMap<u32, EventKlass>,
}

impl EventKlassRegistry {
    pub fn new() -> EventKlassRegistry {
        let mut reg = EventKlassRegistry {
            klasses: std::collections::HashMap::new(),
        };
        reg.create_core_klasses();
        reg
    }

    fn create_core_klass(
        &mut self,
        klass_id: CoreEventKlassId,
        klass_name: &str,
        fields: &[(&str, &str, DataType)],
    ) {
        let mut klass = EventKlass::new(klass_id as u32, klass_name.to_string());
        for (name, type_name, data_type) in fields {
            klass.add_field(name.to_string(), type_name.to_string(), *data_type);
        }
        self.klasses.insert(klass.get_id(), klass);
    }

    fn create_core_klasses(&mut self) {
        self.create_core_klass(
            CoreEventKlassId::Base,
            "HT_Event",
            &[
                ("type", "uint32_t", DataType::U32),
                ("timestamp", "uint64_t", DataType::U64),
                ("id", "uint64_t", DataType::U64),
            ],
        );

        self.create_core_klass(
            CoreEventKlassId::Endianness,
            "HT_EndiannessInfoEvent",
            &[("endianness", "uint8_t", DataType::U8)],
        );

        self.create_core_klass(
            CoreEventKlassId::KlassInfo,
            "HT_EventKlassInfoEvent",
            &[
                ("info_klass_id", "uint32_t", DataType::U32),
                ("event_klass_name", "const char*", DataType::Str),
                ("field_count", "uint8_t", DataType::U8),
            ],
        );

        self.create_core_klass(
            CoreEventKlassId::FieldInfo,
            "HT_EventKlassFieldInfoEvent",
            &[
                ("info_klass_id", "uint32_t", DataType::U32),
                ("field_type", "const char*", DataType::Str),
                ("field_name", "const char*", DataType::Str),
                ("size", "uint64_t", DataType::U64),
                ("data_type", "uint8_t", DataType::U8),
            ],
        );
    }

    pub fn add_klass(&mut self, klass: EventKlass) {
        self.klasses.entry(klass.get_id()).or_insert(klass);
    }

    pub fn get_klass_by_id(&self, id: u32) -> Option<&EventKlass> {
        self.klasses.get(&id)
    }

    pub fn get_klass_by_id_mut(&mut self, id: u32) -> Option<&mut EventKlass> {
        self.klasses.get_mut(&id)
    }

    pub fn get_klass_by_name(&self, name: &str) -> Option<&EventKlass> {
        for (_, klass) in self.klasses.iter() {
            if klass.get_name() == name {
                return Some(klass);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_klass_by_name_should_not_be_none_for_existing_klass() {
        let name = String::from("test_name");
        let mut registry = EventKlassRegistry::new();
        registry.add_klass(EventKlass::new(99, name.clone()));

        assert!(registry.get_klass_by_name(&name).is_some());
    }

    #[test]
    fn get_klass_by_id_should_not_be_none_for_existing_klass() {
        let klass_id = 99;
        let mut registry = EventKlassRegistry::new();
        registry.add_klass(EventKlass::new(klass_id, String::from("test_name")));

        assert!(registry.get_klass_by_id(klass_id).is_some());
        assert!(registry.get_klass_by_id_mut(klass_id).is_some());
    }

    #[test]
    fn get_klass_by_name_should_be_none_if_not_exists() {
        let registry = EventKlassRegistry::new();

        assert!(registry.get_klass_by_name("test").is_none());
    }

    #[test]
    fn check_core_event_klasses() {
        for i in 1..4 {
            assert!(CoreEventKlassId::is_core_klass(i));
        }
        assert!(!CoreEventKlassId::is_core_klass(5));
        assert!(!CoreEventKlassId::is_core_klass(99));
    }
}

use crate::data_provider::DataProvider;
use crate::data_struct_reader::{DataStructReader, ReadEventError};
use crate::event::Event;
use crate::registry::{CoreEventKlassId, EventKlassRegistry};
use crate::registry_updater::RegistryUpdater;

pub struct EventReader {
    data_provider: DataProvider,
}

impl EventReader {
    pub fn new(data_provider: DataProvider) -> EventReader {
        EventReader { data_provider }
    }

    pub fn read_event(
        &mut self,
        registry: &mut EventKlassRegistry,
    ) -> Result<Event, ReadEventError> {
        let base_event = self.read_header(registry)?;

        let klass_id = base_event
            .get_value_u32("type")
            .expect("Cannot find 'type' field in base klass. Registry corrupted?");

        if klass_id == CoreEventKlassId::Base as u32 {
            return Ok(base_event);
        }

        let event = self.read_regular_event(registry, klass_id, base_event)?;

        if klass_id == CoreEventKlassId::KlassInfo as u32
            || klass_id == CoreEventKlassId::FieldInfo as u32
        {
            if let Err(err) = RegistryUpdater::new(registry).update_registry_from_event(&event) {
                return Err(ReadEventError::RegistryUpdateFailed(err.to_owned()));
            }
        }

        Ok(event)
    }

    fn read_regular_event(
        &mut self,
        registry: &EventKlassRegistry,
        klass_id: u32,
        base_event: Event,
    ) -> Result<Event, ReadEventError> {
        let klass = match registry.get_klass_by_id(klass_id) {
            Some(klass) => klass,
            None => return Err(ReadEventError::UnknownKlassId(klass_id)),
        };

        DataStructReader::new(&mut self.data_provider, registry, klass, Some(base_event))
            .read_event()
    }

    fn read_header(&mut self, registry: &mut EventKlassRegistry) -> Result<Event, ReadEventError> {
        let base_event_klass = registry
            .get_klass_by_id(CoreEventKlassId::Base as u32)
            .expect("Can not find Base klass definition!");

        DataStructReader::new(&mut self.data_provider, registry, base_event_klass, None)
            .read_event()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use hawktracer_parser_test_utilities::FakeDataReader;
    use crate::event_klass::EventKlass;
    use crate::event::DataType;

    #[test]
    fn read_header_should_return_valid_base_event() {
        let data = vec![
            1, 0, 0, 0, // type
            1, 2, 0, 0, 0, 0, 0, 0, // timestamp
            2, 0, 0, 0, 0, 0, 0, 0, // id
        ];
        let mut reg = EventKlassRegistry::new();
        let data_provider = DataProvider::new(Box::new(FakeDataReader::new(data, false)));

        let event = EventReader::new(data_provider)
            .read_header(&mut reg)
            .unwrap();

        assert_eq!(event.get_value_u32(&"type").unwrap(), 1);
        assert_eq!(event.get_value_u64(&"timestamp").unwrap(), 513);
        assert_eq!(event.get_value_u64(&"id").unwrap(), 2);
    }


    #[test]
    fn read_event_should_return_full_event() {
        let data = vec![
            100, 0, 0, 0, // type
            1, 2, 0, 0, 0, 0, 0, 0, // timestamp
            2, 0, 0, 0, 0, 0, 0, 0, // id
            65, 66, 67, 0, // ABC
            45, 1, 0, 0, // 301
        ];
        let mut reg = EventKlassRegistry::new();
        let data_provider = DataProvider::new(Box::new(FakeDataReader::new(data, false)));

        let mut klass = EventKlass::new(100, "foo".to_owned());
        klass.add_field("base".to_owned(), "HT_Event".to_owned(), DataType::Struct);
        klass.add_field("str_field".to_owned(), "char*".to_owned(), DataType::Str);
        klass.add_field("u32_field".to_owned(), "uint32_t".to_owned(), DataType::U32);

        reg.add_klass(klass);

        let event = EventReader::new(data_provider)
            .read_event(&mut reg)
            .unwrap();

        assert_eq!(event.get_klass_id(), 100);

        let base_event = event.get_value_struct(&"base").unwrap();
        assert_eq!(base_event.get_value_u32(&"type").unwrap(), 100);
        assert_eq!(base_event.get_value_u64(&"timestamp").unwrap(), 513);
        assert_eq!(base_event.get_value_u64(&"id").unwrap(), 2);

        assert_eq!(event.get_value_string(&"str_field").unwrap(), "ABC");
        assert_eq!(event.get_value_u32(&"u32_field").unwrap(), 301);
    }
}

use crate::data_provider::DataError;
use crate::data_provider::DataProvider;
use crate::event::DataType;
use crate::event::Event;
use crate::event::Value;
use crate::event_klass::EventKlass;
use crate::event_klass::EventKlassField;
use crate::registry::CoreEventKlassId;
use crate::registry::EventKlassRegistry;
use crate::registry_updater::RegistryUpdater;

#[derive(Debug)]
pub enum ReadEventError {
    DataError(DataError),
    UnknownKlass(String),
    UnknownKlassId(u32),
    RegistryUpdateFailed(String),
}

pub struct EventReader {
    data_provider: DataProvider,
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

struct DataStructReader<'a> {
    data_provider: &'a mut DataProvider,
    registry: &'a EventKlassRegistry,
    base_event: Option<Event>,
    klass: &'a EventKlass,
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
        let mut values: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
        for field in klass.get_fields() {
            values.insert(field.get_name().clone(), self.read_field(&field)?);
        }

        Ok(Event::new(klass.get_id(), values))
    }

    fn read_field(&mut self, field: &EventKlassField) -> Result<Value, ReadEventError> {
        match field.get_data_type() {
            DataType::U8 => get_integer!(self, u8, 1, U8),
            DataType::U32 => get_integer!(self, u32, 4, U32),
            DataType::U64 => get_integer!(self, u64, 8, U64),
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

impl EventReader {
    pub fn new(data_provider: DataProvider) -> EventReader {
        EventReader { data_provider }
    }

    pub fn read_event(
        &mut self,
        registry: &mut EventKlassRegistry,
    ) -> Result<Event, ReadEventError> {
        let opt_event = self.read_header(registry)?;
        let base_event = opt_event;

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

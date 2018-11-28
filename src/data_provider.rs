pub struct DataProvider {
    reader: Box<std::io::Read>,
    buffer: [u8; 512],
    data_pointer: usize,
    data_available: usize,
}

#[derive(Debug)]
pub enum DataError {
    EndOfStream,
    Utf8Error,
    IOError(std::io::Error),
}

impl DataProvider {
    pub fn new(reader: Box<std::io::Read>) -> DataProvider {
        DataProvider {
            reader,
            buffer: [0; 512],
            data_pointer: 0,
            data_available: 0,
        }
    }

    fn get_next_byte(&mut self) -> Result<u8, DataError> {
        if self.data_pointer == self.data_available {
            match self.load_data() {
                Err(err) => return Err(DataError::IOError(err)),
                Ok(_) => {
                    if self.data_available == 0 {
                        return Err(DataError::EndOfStream);
                    }
                }
            }
        }

        let data = Ok(self.buffer[self.data_pointer]);
        self.data_pointer += 1;
        data
    }

    pub fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<(), DataError> {
        // TODO do it more efficiently by copying a whole slice
        for b in buffer {
            *b = match self.get_next_byte() {
                Ok(value) => value,
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    pub fn read_string(&mut self) -> Result<String, DataError> {
        let mut data = std::vec::Vec::new();
        loop {
            match self.get_next_byte() {
                Ok(0) => break,
                Ok(b) => data.push(b),
                Err(err) => return Err(err),
            };
        }

        match String::from_utf8(data) {
            Ok(res) => Ok(res),
            Err(_err) => Err(DataError::Utf8Error),
        }
    }

    fn load_data(&mut self) -> std::io::Result<usize> {
        self.data_pointer = 0;
        match self.reader.read(&mut self.buffer) {
            Ok(size) => {
                self.data_available = size;
                Ok(size)
            }
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffers_equal(b1: &[u8], b2: &[u8]) -> usize {
        return b1.iter().zip(b2).map(|(a, b)| assert_eq!(a, b)).count();
    }

    struct FakeDataReader {
        buffer: [u8; 4],
        pointer: usize,
        failing: bool,
    }

    impl FakeDataReader {
        pub fn new(buffer: [u8; 4], failing: bool) -> FakeDataReader {
            FakeDataReader {
                buffer,
                pointer: 0,
                failing,
            }
        }
    }

    impl std::io::Read for FakeDataReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.failing {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Fail"));
            }
            let copy_size = std::cmp::min(buf.len(), self.buffer.len() - self.pointer);
            match copy_size {
                0 => Ok(0),
                _v => {
                    buf[..copy_size]
                        .copy_from_slice(&self.buffer[self.pointer..self.pointer + copy_size]);
                    self.pointer = self.pointer + copy_size;
                    Ok(copy_size)
                }
            }
        }
    }

    #[test]
    fn should_not_set_eos_if_still_have_data() {
        let mut provider = DataProvider::new(Box::new(FakeDataReader::new([1, 2, 3, 4], false)));
        let mut buf = [0u8; 2];
        assert!(provider.read_bytes(&mut buf).is_ok());

        buffers_equal(&buf, &[1, 2]);
    }

    #[test]
    fn should_set_eos_if_try_to_read_too_much_data() {
        let mut provider = DataProvider::new(Box::new(FakeDataReader::new([1, 2, 3, 4], false)));
        let mut buf = [0u8; 5];
        assert!(provider.read_bytes(&mut buf).is_err());

        buffers_equal(&buf[0..4], &[1, 2, 3, 4]);
    }

    #[test]
    fn should_fail_if_data_reader_fails() {
        let mut provider = DataProvider::new(Box::new(FakeDataReader::new([1, 2, 3, 4], true)));
        let mut buf = [0u8; 2];

        assert!(provider.read_bytes(&mut buf).is_err());
    }

    #[test]
    fn read_string_should_not_fail_if_valid_string() {
        let mut provider = DataProvider::new(Box::new(FakeDataReader::new([65, 66, 0, 3], false)));

        let message = provider.read_string();
        assert!(message.is_ok());
        assert_eq!("AB", message.unwrap());
    }

    #[test]
    fn read_string_should_fail_if_no_zero_at_the_end() {
        let mut provider =
            DataProvider::new(Box::new(FakeDataReader::new([65, 66, 67, 68], false)));

        let message = provider.read_string();
        assert!(message.is_err());
    }

    #[test]
    fn read_string_should_fail_if_non_utf8_string() {
        let mut provider = DataProvider::new(Box::new(FakeDataReader::new([65, 220, 0, 5], false)));

        let message = provider.read_string();
        assert!(message.is_err());
    }
}

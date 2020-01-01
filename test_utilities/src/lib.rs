pub struct FakeDataReader {
    buffer: Vec<u8>,
    pointer: usize,
    failing: bool,
}

impl FakeDataReader {
    pub fn new(buffer: Vec<u8>, failing: bool) -> FakeDataReader {
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
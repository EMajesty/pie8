//! Platform-independent, strictly read-only 512-byte block devices.

pub const BLOCK_SIZE: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockError {
    OutOfRange,
    InvalidBuffer,
    Io,
    Timeout,
    Unsupported,
}

pub trait ReadBlockDevice {
    fn block_count(&self) -> u64;
    fn read_blocks(&self, first: u64, destination: &mut [u8]) -> Result<(), BlockError>;
}

pub struct MemoryBlockDevice<'a> {
    bytes: &'a [u8],
}
impl<'a> MemoryBlockDevice<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, BlockError> {
        if bytes.len() % BLOCK_SIZE != 0 {
            return Err(BlockError::InvalidBuffer);
        }
        Ok(Self { bytes })
    }
}
impl ReadBlockDevice for MemoryBlockDevice<'_> {
    fn block_count(&self) -> u64 {
        (self.bytes.len() / BLOCK_SIZE) as u64
    }
    fn read_blocks(&self, first: u64, destination: &mut [u8]) -> Result<(), BlockError> {
        if destination.is_empty() || destination.len() % BLOCK_SIZE != 0 {
            return Err(BlockError::InvalidBuffer);
        }
        let start = usize::try_from(first)
            .ok()
            .and_then(|n| n.checked_mul(BLOCK_SIZE))
            .ok_or(BlockError::OutOfRange)?;
        let end = start
            .checked_add(destination.len())
            .ok_or(BlockError::OutOfRange)?;
        let source = self.bytes.get(start..end).ok_or(BlockError::OutOfRange)?;
        destination.copy_from_slice(source);
        Ok(())
    }
}

#[cfg(feature = "std")]
pub mod host {
    use super::*;
    use std::{
        cell::RefCell,
        fs::File,
        io::{Read, Seek, SeekFrom},
        path::Path,
    };
    pub struct ImageBlockDevice {
        file: RefCell<File>,
        blocks: u64,
    }
    impl ImageBlockDevice {
        pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
            let file = File::open(path)?;
            let size = file.metadata()?.len();
            if size % BLOCK_SIZE as u64 != 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "image is not block aligned",
                ));
            }
            Ok(Self {
                file: RefCell::new(file),
                blocks: size / BLOCK_SIZE as u64,
            })
        }
    }
    impl ReadBlockDevice for ImageBlockDevice {
        fn block_count(&self) -> u64 {
            self.blocks
        }
        fn read_blocks(&self, first: u64, destination: &mut [u8]) -> Result<(), BlockError> {
            if destination.is_empty() || destination.len() % BLOCK_SIZE != 0 {
                return Err(BlockError::InvalidBuffer);
            }
            let mut file = self.file.borrow_mut();
            file.seek(SeekFrom::Start(
                first
                    .checked_mul(BLOCK_SIZE as u64)
                    .ok_or(BlockError::OutOfRange)?,
            ))
            .map_err(|_| BlockError::Io)?;
            file.read_exact(destination).map_err(|_| BlockError::Io)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memory_device_checks_boundaries() {
        let image = [7; 1024];
        let device = MemoryBlockDevice::new(&image).unwrap();
        let mut block = [0; 512];
        device.read_blocks(1, &mut block).unwrap();
        assert_eq!(block[0], 7);
        assert_eq!(
            device.read_blocks(2, &mut block),
            Err(BlockError::OutOfRange)
        );
        assert_eq!(
            device.read_blocks(0, &mut block[..1]),
            Err(BlockError::InvalidBuffer)
        );
    }
}

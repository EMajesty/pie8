use crate::block::{BLOCK_SIZE, BlockError, ReadBlockDevice};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Partition {
    pub first_lba: u32,
    pub sectors: u32,
    pub partition_type: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PartitionError {
    Block(BlockError),
    InvalidMbr,
    NoFat32,
    OutOfRange,
}

pub fn first_fat32(device: &impl ReadBlockDevice) -> Result<Partition, PartitionError> {
    let mut sector = [0; BLOCK_SIZE];
    device
        .read_blocks(0, &mut sector)
        .map_err(PartitionError::Block)?;
    if sector[510..512] != [0x55, 0xaa] {
        return Err(PartitionError::InvalidMbr);
    }
    for index in 0..4 {
        let entry = &sector[446 + index * 16..];
        let kind = entry[4];
        if matches!(kind, 0x0b | 0x0c) {
            let first_lba = u32::from_le_bytes(entry[8..12].try_into().unwrap());
            let sectors = u32::from_le_bytes(entry[12..16].try_into().unwrap());
            let end = u64::from(first_lba)
                .checked_add(u64::from(sectors))
                .ok_or(PartitionError::OutOfRange)?;
            if sectors == 0 || end > device.block_count() {
                return Err(PartitionError::OutOfRange);
            }
            return Ok(Partition {
                first_lba,
                sectors,
                partition_type: kind,
            });
        }
    }
    Err(PartitionError::NoFat32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::MemoryBlockDevice;
    #[test]
    fn discovers_and_bounds_fat32() {
        let mut image = [0; 1024];
        image[510] = 0x55;
        image[511] = 0xaa;
        image[450] = 0x0c;
        image[454..458].copy_from_slice(&1_u32.to_le_bytes());
        image[458..462].copy_from_slice(&1_u32.to_le_bytes());
        let p = first_fat32(&MemoryBlockDevice::new(&image).unwrap()).unwrap();
        assert_eq!(p.first_lba, 1);
        image[458..462].copy_from_slice(&2_u32.to_le_bytes());
        assert_eq!(
            first_fat32(&MemoryBlockDevice::new(&image).unwrap()),
            Err(PartitionError::OutOfRange)
        );
    }
    #[test]
    fn rejects_malformed_mbr() {
        let image = [0; 512];
        assert_eq!(
            first_fat32(&MemoryBlockDevice::new(&image).unwrap()),
            Err(PartitionError::InvalidMbr)
        );
    }
}

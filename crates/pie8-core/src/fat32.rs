//! Bounded, read-only FAT32 traversal. No operation can modify the device.
use crate::{
    block::{BLOCK_SIZE, BlockError, ReadBlockDevice},
    partition::Partition,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FatError {
    Block(BlockError),
    InvalidBootSector,
    Unsupported,
    Corrupt,
    NotFound,
    BufferTooSmall,
}

#[derive(Clone, Copy)]
pub struct DirectoryEntry {
    name: [u8; 256],
    name_len: u8,
    pub attributes: u8,
    pub first_cluster: u32,
    pub size: u32,
}
impl DirectoryEntry {
    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("?")
    }
    pub fn is_directory(&self) -> bool {
        self.attributes & 0x10 != 0
    }
    pub fn is_wav(&self) -> bool {
        self.name()
            .as_bytes()
            .get((self.name_len as usize).saturating_sub(4)..)
            .is_some_and(|e| e.eq_ignore_ascii_case(b".wav"))
    }
}

pub struct Fat32<'a, D> {
    device: &'a D,
    partition: Partition,
    fat_lba: u64,
    data_lba: u64,
    sectors_per_cluster: u8,
    root_cluster: u32,
    cluster_count: u32,
}

impl<'a, D: ReadBlockDevice> Fat32<'a, D> {
    pub fn mount(device: &'a D, partition: Partition) -> Result<Self, FatError> {
        let mut b = [0; BLOCK_SIZE];
        device
            .read_blocks(partition.first_lba.into(), &mut b)
            .map_err(FatError::Block)?;
        if b[510..512] != [0x55, 0xaa] || u16::from_le_bytes([b[11], b[12]]) != 512 {
            return Err(FatError::InvalidBootSector);
        }
        let spc = b[13];
        let reserved = u16::from_le_bytes([b[14], b[15]]) as u32;
        let fats = b[16] as u32;
        let root_entries = u16::from_le_bytes([b[17], b[18]]);
        let fat16 = u16::from_le_bytes([b[22], b[23]]);
        let total = u32::from_le_bytes(b[32..36].try_into().unwrap());
        let fat_size = u32::from_le_bytes(b[36..40].try_into().unwrap());
        let root = u32::from_le_bytes(b[44..48].try_into().unwrap());
        if spc == 0
            || !spc.is_power_of_two()
            || reserved == 0
            || fats == 0
            || root_entries != 0
            || fat16 != 0
            || total == 0
            || fat_size == 0
            || root < 2
            || total > partition.sectors
        {
            return Err(FatError::Unsupported);
        }
        let overhead = reserved
            .checked_add(fats.checked_mul(fat_size).ok_or(FatError::Corrupt)?)
            .ok_or(FatError::Corrupt)?;
        let data = total.checked_sub(overhead).ok_or(FatError::Corrupt)?;
        let clusters = data / u32::from(spc);
        if clusters == 0 {
            return Err(FatError::Unsupported);
        }
        let fat_start = partition
            .first_lba
            .checked_add(reserved)
            .ok_or(FatError::Corrupt)?;
        let data_start = partition
            .first_lba
            .checked_add(overhead)
            .ok_or(FatError::Corrupt)?;
        Ok(Self {
            device,
            partition,
            fat_lba: u64::from(fat_start),
            data_lba: u64::from(data_start),
            sectors_per_cluster: spc,
            root_cluster: root,
            cluster_count: clusters,
        })
    }
    pub fn partition(&self) -> Partition {
        self.partition
    }
    pub fn cluster_count(&self) -> u32 {
        self.cluster_count
    }
    fn cluster_lba(&self, c: u32) -> Result<u64, FatError> {
        if c < 2 || c - 2 >= self.cluster_count {
            return Err(FatError::Corrupt);
        }
        Ok(self.data_lba + u64::from(c - 2) * u64::from(self.sectors_per_cluster))
    }
    fn next_cluster(&self, c: u32) -> Result<Option<u32>, FatError> {
        let offset = u64::from(c) * 4;
        let mut b = [0; BLOCK_SIZE];
        self.device
            .read_blocks(self.fat_lba + offset / 512, &mut b)
            .map_err(FatError::Block)?;
        let i = (offset % 512) as usize;
        let value = u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) & 0x0fff_ffff;
        match value {
            0x0fff_fff8..=0x0fff_ffff => Ok(None),
            0x0fff_fff7 => Err(FatError::Corrupt),
            2..=0x0fff_ffef => Ok(Some(value)),
            _ => Err(FatError::Corrupt),
        }
    }
    pub fn enumerate(
        &self,
        directory: Option<&DirectoryEntry>,
        mut visitor: impl FnMut(&DirectoryEntry) -> bool,
    ) -> Result<(), FatError> {
        let mut cluster = directory.map_or(self.root_cluster, |e| e.first_cluster);
        let mut traversed = 0;
        let mut lfn = [0u8; 256];
        let mut lfn_len = 0usize;
        let mut sector = [0; BLOCK_SIZE];
        loop {
            traversed += 1;
            if traversed > self.cluster_count {
                return Err(FatError::Corrupt);
            }
            let lba = self.cluster_lba(cluster)?;
            for s in 0..u64::from(self.sectors_per_cluster) {
                self.device
                    .read_blocks(lba + s, &mut sector)
                    .map_err(FatError::Block)?;
                for raw in sector.chunks_exact(32) {
                    if raw[0] == 0 {
                        return Ok(());
                    }
                    if raw[0] == 0xe5 {
                        lfn_len = 0;
                        continue;
                    }
                    if raw[11] == 0x0f {
                        let sequence = (raw[0] & 0x1f) as usize;
                        if sequence == 0 || sequence > 20 {
                            lfn_len = 0;
                            continue;
                        }
                        let base = (sequence - 1) * 13;
                        for (n, p) in [1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30]
                            .into_iter()
                            .enumerate()
                        {
                            let ch = u16::from_le_bytes([raw[p], raw[p + 1]]);
                            if ch != 0 && ch != 0xffff && ch < 128 && base + n < 255 {
                                lfn[base + n] = ch as u8;
                                lfn_len = lfn_len.max(base + n + 1)
                            }
                        }
                        continue;
                    }
                    if raw[11] & 0x08 != 0 {
                        lfn_len = 0;
                        continue;
                    }
                    let mut entry = DirectoryEntry {
                        name: [0; 256],
                        name_len: 0,
                        attributes: raw[11],
                        first_cluster: (u32::from(u16::from_le_bytes([raw[20], raw[21]])) << 16)
                            | u32::from(u16::from_le_bytes([raw[26], raw[27]])),
                        size: u32::from_le_bytes(raw[28..32].try_into().unwrap()),
                    };
                    if lfn_len > 0 {
                        entry.name[..lfn_len].copy_from_slice(&lfn[..lfn_len]);
                        entry.name_len = lfn_len as u8
                    } else {
                        entry.name_len = short_name(raw, &mut entry.name) as u8
                    }
                    lfn_len = 0;
                    if !visitor(&entry) {
                        return Ok(());
                    }
                }
            }
            cluster = match self.next_cluster(cluster)? {
                Some(n) => n,
                None => return Ok(()),
            };
        }
    }
    pub fn find_path(&self, path: &str) -> Result<DirectoryEntry, FatError> {
        let mut directory = None;
        let mut found = None;
        for component in path.split('/').filter(|s| !s.is_empty()) {
            found = None;
            self.enumerate(directory.as_ref(), |e| {
                if e.name().eq_ignore_ascii_case(component) {
                    found = Some(*e);
                    false
                } else {
                    true
                }
            })?;
            let entry = found.ok_or(FatError::NotFound)?;
            directory = Some(entry)
        }
        found.ok_or(FatError::NotFound)
    }
    pub fn read_file(
        &self,
        entry: &DirectoryEntry,
        destination: &mut [u8],
    ) -> Result<usize, FatError> {
        if entry.is_directory() {
            return Err(FatError::Unsupported);
        }
        let size = entry.size as usize;
        if destination.len() < size {
            return Err(FatError::BufferTooSmall);
        }
        if size == 0 {
            return Ok(0);
        }
        let mut cluster = entry.first_cluster;
        let mut written = 0;
        let mut traversed = 0;
        let mut sector = [0; BLOCK_SIZE];
        while written < size {
            traversed += 1;
            if traversed > self.cluster_count {
                return Err(FatError::Corrupt);
            }
            let lba = self.cluster_lba(cluster)?;
            for s in 0..u64::from(self.sectors_per_cluster) {
                self.device
                    .read_blocks(lba + s, &mut sector)
                    .map_err(FatError::Block)?;
                let count = (size - written).min(512);
                destination[written..written + count].copy_from_slice(&sector[..count]);
                written += count;
                if written == size {
                    return Ok(size);
                }
            }
            cluster = self.next_cluster(cluster)?.ok_or(FatError::Corrupt)?
        }
        Ok(size)
    }
}
fn short_name(raw: &[u8], out: &mut [u8; 256]) -> usize {
    let mut n = 0;
    for &c in &raw[..8] {
        if c != b' ' {
            out[n] = c;
            n += 1
        }
    }
    if raw[8..11].iter().any(|&c| c != b' ') {
        out[n] = b'.';
        n += 1;
        for &c in &raw[8..11] {
            if c != b' ' {
                out[n] = c;
                n += 1
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::MemoryBlockDevice;
    fn image(fragmented: bool) -> [u8; 512 * 8] {
        let mut i = [0u8; 512 * 8];
        i[510] = 0x55;
        i[511] = 0xaa;
        i[450] = 0x0c;
        i[454..458].copy_from_slice(&1u32.to_le_bytes());
        i[458..462].copy_from_slice(&7u32.to_le_bytes());
        let b = &mut i[512..1024];
        b[11..13].copy_from_slice(&512u16.to_le_bytes());
        b[13] = 1;
        b[14..16].copy_from_slice(&1u16.to_le_bytes());
        b[16] = 1;
        b[32..36].copy_from_slice(&7u32.to_le_bytes());
        b[36..40].copy_from_slice(&1u32.to_le_bytes());
        b[44..48].copy_from_slice(&2u32.to_le_bytes());
        b[510] = 0x55;
        b[511] = 0xaa;
        let fat = &mut i[1024..1536];
        fat[8..12].copy_from_slice(&0x0fff_ffffu32.to_le_bytes());
        fat[12..16]
            .copy_from_slice(&(if fragmented { 5_u32 } else { 0x0fff_ffff_u32 }).to_le_bytes());
        fat[20..24].copy_from_slice(&0x0fff_ffffu32.to_le_bytes());
        let d = &mut i[1536..2048];
        d[..11].copy_from_slice(b"TEST    WAV");
        d[11] = 0x20;
        d[26..28].copy_from_slice(&3u16.to_le_bytes());
        d[28..32].copy_from_slice(&(if fragmented { 600_u32 } else { 4_u32 }).to_le_bytes());
        i[2048..2052].copy_from_slice(b"RIFF");
        if fragmented {
            i[3072..3160].fill(9)
        }
        i
    }
    #[test]
    fn traverses_root_and_reads_file() {
        let i = image(false);
        let d = MemoryBlockDevice::new(&i).unwrap();
        let p = crate::partition::first_fat32(&d).unwrap();
        let fs = Fat32::mount(&d, p).unwrap();
        let e = fs.find_path("test.wav").unwrap();
        let mut out = [0; 4];
        fs.read_file(&e, &mut out).unwrap();
        assert_eq!(&out, b"RIFF")
    }
    #[test]
    fn follows_fragmented_chain() {
        let i = image(true);
        let d = MemoryBlockDevice::new(&i).unwrap();
        let fs = Fat32::mount(&d, crate::partition::first_fat32(&d).unwrap()).unwrap();
        let e = fs.find_path("TEST.WAV").unwrap();
        let mut out = [0; 600];
        fs.read_file(&e, &mut out).unwrap();
        assert_eq!(out[512], 9)
    }
    #[test]
    fn malformed_chain_is_rejected() {
        let mut i = image(true);
        i[1024 + 12..1024 + 16].copy_from_slice(&1u32.to_le_bytes());
        let d = MemoryBlockDevice::new(&i).unwrap();
        let fs = Fat32::mount(&d, crate::partition::first_fat32(&d).unwrap()).unwrap();
        let e = fs.find_path("TEST.WAV").unwrap();
        assert_eq!(fs.read_file(&e, &mut [0; 600]), Err(FatError::Corrupt))
    }
}

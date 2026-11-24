//! Controller-independent SD response decoding and address calculations.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Addressing {
    Byte,
    Block,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ocr {
    pub ready: bool,
    pub addressing: Addressing,
    pub raw: u32,
}

#[must_use]
pub const fn decode_ocr(raw: u32) -> Ocr {
    Ocr {
        ready: raw & (1 << 31) != 0,
        addressing: if raw & (1 << 30) != 0 {
            Addressing::Block
        } else {
            Addressing::Byte
        },
        raw,
    }
}

/// Returns the capacity in 512-byte logical blocks from a 128-bit CSD.
/// Words are ordered most-significant first, as printed in the SD specification.
pub fn capacity_blocks(csd: [u32; 4]) -> Option<u64> {
    let structure = bits(csd, 127, 126)? as u8;
    match structure {
        0 => {
            let read_bl_len = bits(csd, 83, 80)? as u32;
            let c_size = bits(csd, 73, 62)?;
            let c_size_mult = bits(csd, 49, 47)? as u32;
            if read_bl_len > 31 || c_size_mult > 7 {
                return None;
            }
            let bytes = (c_size + 1)
                .checked_mul(1u64 << (c_size_mult + 2))?
                .checked_mul(1u64 << read_bl_len)?;
            (bytes % 512 == 0).then_some(bytes / 512)
        }
        1 => bits(csd, 69, 48)?.checked_add(1)?.checked_mul(1024),
        _ => None,
    }
}

/// Converts a logical block number to the argument used by CMD17.
pub fn read_argument(block: u64, addressing: Addressing) -> Option<u32> {
    let value = match addressing {
        Addressing::Block => block,
        Addressing::Byte => block.checked_mul(512)?,
    };
    u32::try_from(value).ok()
}

/// Computes the SDHOST divider register and achieved frequency. SDHOST stores
/// `divisor - 2`, unlike the Arasan controller.
pub fn sdhost_clock(source_hz: u32, requested_hz: u32) -> Option<(u32, u32)> {
    if source_hz == 0 || requested_hz == 0 {
        return None;
    }
    let divisor = source_hz.div_ceil(requested_hz).clamp(2, 0x801);
    Some((divisor - 2, source_hz / divisor))
}

/// Extracts the assigned RCA from a CMD3 R6 response.
#[must_use]
pub const fn relative_card_address(r6: u32) -> Option<u16> {
    let rca = (r6 >> 16) as u16;
    if rca == 0 { None } else { Some(rca) }
}

fn bits(words: [u32; 4], high: u8, low: u8) -> Option<u64> {
    if high > 127 || low > high || high - low >= 64 {
        return None;
    }
    let mut value = 0u64;
    for bit in (low..=high).rev() {
        let word = 3usize.checked_sub((bit / 32) as usize)?;
        value = (value << 1) | u64::from((words[word] >> (bit % 32)) & 1);
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_bits(words: &mut [u32; 4], high: u8, low: u8, value: u64) {
        for bit in low..=high {
            let word = 3 - (bit / 32) as usize;
            let source = bit - low;
            words[word] |= ((value >> source) as u32 & 1) << (bit % 32);
        }
    }

    #[test]
    fn decodes_ocr_and_addressing() {
        assert_eq!(
            decode_ocr(0),
            Ocr {
                ready: false,
                addressing: Addressing::Byte,
                raw: 0
            }
        );
        assert_eq!(decode_ocr(0xc000_0000).addressing, Addressing::Block);
        assert_eq!(read_argument(7, Addressing::Block), Some(7));
        assert_eq!(read_argument(7, Addressing::Byte), Some(3584));
        assert_eq!(read_argument(u64::from(u32::MAX), Addressing::Byte), None);
    }

    #[test]
    fn calculates_sd_hc_capacity() {
        let mut csd = [0; 4];
        set_bits(&mut csd, 127, 126, 1);
        set_bits(&mut csd, 69, 48, 30_719); // 15 GiB
        assert_eq!(capacity_blocks(csd), Some(31_457_280));
    }

    #[test]
    fn calculates_standard_capacity() {
        let mut csd = [0; 4];
        set_bits(&mut csd, 83, 80, 9);
        set_bits(&mut csd, 73, 62, 1023);
        set_bits(&mut csd, 49, 47, 7);
        assert_eq!(capacity_blocks(csd), Some(524_288));
    }

    #[test]
    fn rejects_unknown_csd_structure() {
        let mut csd = [0; 4];
        set_bits(&mut csd, 127, 126, 2);
        assert_eq!(capacity_blocks(csd), None);
    }

    #[test]
    fn calculates_sdhost_clock_and_decodes_r6() {
        assert_eq!(sdhost_clock(250_000_000, 400_000), Some((623, 400_000)));
        assert_eq!(sdhost_clock(250_000_000, 25_000_000), Some((8, 25_000_000)));
        assert_eq!(sdhost_clock(0, 400_000), None);
        assert_eq!(relative_card_address(0x1234_0000), Some(0x1234));
        assert_eq!(relative_card_address(0), None);
    }
}

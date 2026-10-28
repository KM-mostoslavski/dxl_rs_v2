//! CRC-16 of Protocol 2 packets.
//!
//! Polynomial 0x8005, initial value 0, not reflected, no final XOR, as given
//! by the e-Manual (<https://emanual.robotis.com/docs/en/dxl/crc/>). The
//! table is built at compile time from the polynomial.

const POLY: u16 = 0x8005;

const TABLE: [u16; 256] = table();

const fn table() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = (i as u16) << 8;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ POLY
            } else {
                crc << 1
            };
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

/// Feeds one byte into a running CRC.
pub(crate) const fn update_byte(crc: u16, byte: u8) -> u16 {
    (crc << 8) ^ TABLE[((crc >> 8) as u8 ^ byte) as usize]
}

/// Feeds `bytes` into a running CRC; start from 0.
pub(crate) fn update(crc: u16, bytes: &[u8]) -> u16 {
    bytes.iter().fold(crc, |crc, &b| update_byte(crc, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_emanual_start() {
        assert_eq!(&TABLE[..4], &[0x0000, 0x8005, 0x800F, 0x000A]);
    }

    #[test]
    fn catalogue_check_value() {
        assert_eq!(update(0, b"123456789"), 0xFEE8);
    }

    #[test]
    fn emanual_ping() {
        assert_eq!(
            update(0, &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x03, 0x00, 0x01]),
            0x4E19
        );
    }
}

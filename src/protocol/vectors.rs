//! Packet examples copied from the ROBOTIS e-Manual, fetched 2026-10-26.
//!
//! The e-Manual is being retired, so the bytes live here. Each constant names
//! its page and anchor. "Integrity vector only" marks examples whose
//! instruction this crate does not send; they still check the CRC/checksum.

/// <https://emanual.robotis.com/docs/en/dxl/protocol2/>
pub(crate) mod p2 {
    /// `#example-1`: Ping ID 1.
    pub(crate) const PING: &[u8] = &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x03, 0x00, 0x01, 0x19, 0x4E];
    /// `#example-1`: ID 1 replies model 1030 (XM430-W210), firmware 38.
    pub(crate) const PING_REPLY: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
    ];
    /// `#example-2`: broadcast Ping.
    pub(crate) const BROADCAST_PING: &[u8] =
        &[0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x03, 0x00, 0x01, 0x31, 0x42];
    /// `#example-2`: ID 2 replies model 1030, firmware 38. ID 1's reply is
    /// [`PING_REPLY`].
    pub(crate) const BROADCAST_PING_REPLY_2: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x02, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x6F, 0x6D,
    ];
    /// `#example`: Read Present Position (address 132, 4 bytes) from ID 1.
    pub(crate) const READ: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x02, 0x84, 0x00, 0x04, 0x00, 0x1D, 0x15,
    ];
    /// `#example`: ID 1 replies 166 (`A6 00 00 00`).
    pub(crate) const READ_REPLY_1: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x08, 0x00, 0x55, 0x00, 0xA6, 0x00, 0x00, 0x00, 0x8C, 0xC0,
    ];
    /// Sync Read example: ID 2 replies 2079 (`1F 08 00 00`).
    pub(crate) const READ_REPLY_2: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x02, 0x08, 0x00, 0x55, 0x00, 0x1F, 0x08, 0x00, 0x00, 0xBA, 0xBE,
    ];
    /// Write Goal Position (address 116) = 512 to ID 1.
    pub(crate) const WRITE: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x09, 0x00, 0x03, 0x74, 0x00, 0x00, 0x02, 0x00, 0x00, 0xCA,
        0x89,
    ];
    /// Status from ID 1: no error, no data. Reply to Write and to every
    /// integrity-only example below.
    pub(crate) const OK_REPLY: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x04, 0x00, 0x55, 0x00, 0xA1, 0x0C,
    ];
    /// Sync Read Present Position (132, 4 bytes) from IDs 1 and 2; replies
    /// are [`READ_REPLY_1`] then [`READ_REPLY_2`].
    pub(crate) const SYNC_READ: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x09, 0x00, 0x82, 0x84, 0x00, 0x04, 0x00, 0x01, 0x02, 0xCE,
        0xFA,
    ];
    /// Sync Write Goal Position (116, 4 bytes): ID 1 = 150, ID 2 = 170.
    pub(crate) const SYNC_WRITE: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x11, 0x00, 0x83, 0x74, 0x00, 0x04, 0x00, 0x01, 0x96, 0x00,
        0x00, 0x00, 0x02, 0xAA, 0x00, 0x00, 0x00, 0x82, 0x87,
    ];

    /// Integrity vector only: Reg Write Goal Velocity (104) = 200, ID 1.
    pub(crate) const REG_WRITE: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x09, 0x00, 0x04, 0x68, 0x00, 0xC8, 0x00, 0x00, 0x00, 0xAE,
        0x8E,
    ];
    /// Integrity vector only: Action, ID 1.
    pub(crate) const ACTION: &[u8] = &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x03, 0x00, 0x05, 0x02, 0xCE];
    /// Integrity vector only: Factory Reset ID 1, option 0x01.
    pub(crate) const FACTORY_RESET: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x04, 0x00, 0x06, 0x01, 0xA1, 0xE6,
    ];
    /// Integrity vector only: Reboot, ID 1.
    pub(crate) const REBOOT: &[u8] = &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x03, 0x00, 0x08, 0x2F, 0x4E];
    /// Integrity vector only: Clear multi-turn, ID 1.
    pub(crate) const CLEAR: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x08, 0x00, 0x10, 0x01, 0x44, 0x58, 0x4C, 0x22, 0xB1, 0xDC,
    ];
    /// Integrity vector only: Control Table Backup store, ID 1.
    pub(crate) const BACKUP_STORE: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x08, 0x00, 0x20, 0x01, 0x43, 0x54, 0x52, 0x4C, 0x16, 0xF5,
    ];
    /// Integrity vector only: Control Table Backup restore, ID 1.
    pub(crate) const BACKUP_RESTORE: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x08, 0x00, 0x20, 0x02, 0x43, 0x54, 0x52, 0x4C, 0x9E, 0xF5,
    ];
    /// Integrity vector only: Fast Sync Read Present Position, IDs 3, 7, 4.
    pub(crate) const FAST_SYNC_READ: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x0A, 0x00, 0x8A, 0x84, 0x00, 0x04, 0x00, 0x03, 0x07, 0x04,
        0x20, 0xF2,
    ];
    /// Integrity vector only: merged Fast Sync Read reply (166, 2079, 1023).
    pub(crate) const FAST_SYNC_READ_REPLY: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x19, 0x00, 0x55, 0x00, 0x03, 0xA6, 0x00, 0x00, 0x00, 0x84,
        0x08, 0x00, 0x07, 0x1F, 0x08, 0x00, 0x00, 0x16, 0xCA, 0x00, 0x04, 0xFF, 0x03, 0x00, 0x00,
        0xD1, 0x9E,
    ];
    /// Integrity vector only: Bulk Read, ID 1 Present Voltage, ID 2 Present
    /// Temperature.
    pub(crate) const BULK_READ: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x0D, 0x00, 0x92, 0x01, 0x90, 0x00, 0x02, 0x00, 0x02, 0x92,
        0x00, 0x01, 0x00, 0x1A, 0x05,
    ];
    /// Integrity vector only: Bulk Read reply from ID 1 (119).
    pub(crate) const BULK_READ_REPLY_1: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x06, 0x00, 0x55, 0x00, 0x77, 0x00, 0xC3, 0x69,
    ];
    /// Integrity vector only: Bulk Read reply from ID 2 (36).
    pub(crate) const BULK_READ_REPLY_2: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0x02, 0x05, 0x00, 0x55, 0x00, 0x24, 0x8B, 0xA9,
    ];
    /// Integrity vector only: Fast Bulk Read, IDs 3, 7, 4.
    pub(crate) const FAST_BULK_READ: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x12, 0x00, 0x9A, 0x03, 0x84, 0x00, 0x04, 0x00, 0x07, 0x7C,
        0x00, 0x02, 0x00, 0x04, 0x92, 0x00, 0x01, 0x00, 0xDA, 0x2D,
    ];
    /// Integrity vector only: merged Fast Bulk Read reply.
    pub(crate) const FAST_BULK_READ_REPLY: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x14, 0x00, 0x55, 0x00, 0x03, 0xA6, 0x00, 0x00, 0x00, 0x67,
        0xA4, 0x00, 0x07, 0xA5, 0x01, 0x24, 0x74, 0x00, 0x04, 0x1F, 0xD9, 0xC1,
    ];
    /// Integrity vector only: Bulk Write, ID 1 Max Voltage Limit = 160, ID 2
    /// Temperature Limit = 80.
    pub(crate) const BULK_WRITE: &[u8] = &[
        0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x10, 0x00, 0x93, 0x01, 0x20, 0x00, 0x02, 0x00, 0xA0, 0x00,
        0x02, 0x1F, 0x00, 0x01, 0x00, 0x50, 0xB7, 0x68,
    ];

    /// Status packets this crate parses.
    pub(crate) const STATUS: &[&[u8]] = &[
        PING_REPLY,
        BROADCAST_PING_REPLY_2,
        READ_REPLY_1,
        READ_REPLY_2,
        OK_REPLY,
    ];

    /// Every example on the page.
    pub(crate) const ALL: &[&[u8]] = &[
        PING,
        PING_REPLY,
        BROADCAST_PING,
        BROADCAST_PING_REPLY_2,
        READ,
        READ_REPLY_1,
        READ_REPLY_2,
        WRITE,
        OK_REPLY,
        SYNC_READ,
        SYNC_WRITE,
        REG_WRITE,
        ACTION,
        FACTORY_RESET,
        REBOOT,
        CLEAR,
        BACKUP_STORE,
        BACKUP_RESTORE,
        FAST_SYNC_READ,
        FAST_SYNC_READ_REPLY,
        BULK_READ,
        BULK_READ_REPLY_1,
        BULK_READ_REPLY_2,
        FAST_BULK_READ,
        FAST_BULK_READ_REPLY,
        BULK_WRITE,
    ];
}

/// <https://emanual.robotis.com/docs/en/dxl/protocol1/>
pub(crate) mod p1 {
    /// `#ping`: Ping ID 1 (RX-64).
    pub(crate) const PING: &[u8] = &[0xFF, 0xFF, 0x01, 0x02, 0x01, 0xFB];
    /// `#ping`: ID 1 replies with no error. Also the reply to Reg Write and
    /// Reboot.
    pub(crate) const OK_REPLY: &[u8] = &[0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC];
    /// Read Present Temperature (address 43, 1 byte) from ID 1.
    pub(crate) const READ: &[u8] = &[0xFF, 0xFF, 0x01, 0x04, 0x02, 0x2B, 0x01, 0xCC];
    /// ID 1 replies 32 (`0x20`).
    pub(crate) const READ_REPLY: &[u8] = &[0xFF, 0xFF, 0x01, 0x03, 0x00, 0x20, 0xDB];
    /// `#checksum-instruction-packet`: Write address 12 = 100, 170 to ID 1
    /// (the checksum worked example).
    pub(crate) const WRITE: &[u8] = &[0xFF, 0xFF, 0x01, 0x05, 0x03, 0x0C, 0x64, 0xAA, 0xDC];
    /// Write ID = 1 via broadcast; no reply.
    pub(crate) const WRITE_BROADCAST: &[u8] = &[0xFF, 0xFF, 0xFE, 0x04, 0x03, 0x03, 0x01, 0xF6];
    /// Sync Write MX-64, 4 bytes from address 30: ID 0 = `10 00 50 01`,
    /// ID 1 = `20 02 60 03`.
    pub(crate) const SYNC_WRITE: &[u8] = &[
        0xFF, 0xFF, 0xFE, 0x0E, 0x83, 0x1E, 0x04, 0x00, 0x10, 0x00, 0x50, 0x01, 0x01, 0x20, 0x02,
        0x60, 0x03, 0x67,
    ];
    /// `#error`: ID 1 reports Overload and Overheating.
    pub(crate) const ERROR_REPLY: &[u8] = &[0xFF, 0xFF, 0x01, 0x02, 0x24, 0xD8];

    /// Integrity vector only: Reg Write Goal Position (30) = 500, ID 1.
    pub(crate) const REG_WRITE: &[u8] = &[0xFF, 0xFF, 0x01, 0x05, 0x04, 0x1E, 0xF4, 0x01, 0xE2];
    /// Integrity vector only: Action, broadcast.
    pub(crate) const ACTION: &[u8] = &[0xFF, 0xFF, 0xFE, 0x02, 0x05, 0xFA];
    /// Integrity vector only: Factory Reset, ID 0.
    pub(crate) const FACTORY_RESET: &[u8] = &[0xFF, 0xFF, 0x00, 0x02, 0x06, 0xF7];
    /// Integrity vector only: reply to Factory Reset from ID 0.
    pub(crate) const FACTORY_RESET_REPLY: &[u8] = &[0xFF, 0xFF, 0x00, 0x02, 0x00, 0xFD];
    /// Integrity vector only: Reboot, ID 1.
    pub(crate) const REBOOT: &[u8] = &[0xFF, 0xFF, 0x01, 0x02, 0x08, 0xF4];
    /// Integrity vector only: Bulk Read, ID 1 Goal Position, ID 2 Present
    /// Position.
    pub(crate) const BULK_READ: &[u8] = &[
        0xFF, 0xFF, 0xFE, 0x09, 0x92, 0x00, 0x02, 0x01, 0x1E, 0x02, 0x02, 0x24, 0x1D,
    ];
    /// Integrity vector only: Bulk Read reply from ID 1.
    pub(crate) const BULK_READ_REPLY_1: &[u8] = &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x00, 0x80, 0x7A];
    /// Integrity vector only: Bulk Read reply from ID 2.
    pub(crate) const BULK_READ_REPLY_2: &[u8] = &[0xFF, 0xFF, 0x02, 0x04, 0x00, 0x00, 0x80, 0x79];

    /// Status packets this crate parses.
    pub(crate) const STATUS: &[&[u8]] = &[OK_REPLY, READ_REPLY, ERROR_REPLY, FACTORY_RESET_REPLY];

    /// Every example on the page.
    pub(crate) const ALL: &[&[u8]] = &[
        PING,
        OK_REPLY,
        READ,
        READ_REPLY,
        WRITE,
        WRITE_BROADCAST,
        SYNC_WRITE,
        ERROR_REPLY,
        REG_WRITE,
        ACTION,
        FACTORY_RESET,
        FACTORY_RESET_REPLY,
        REBOOT,
        BULK_READ,
        BULK_READ_REPLY_1,
        BULK_READ_REPLY_2,
    ];
}

#[cfg(test)]
mod tests {
    use super::{p1, p2};
    use crate::protocol::crc;

    #[test]
    fn every_p2_vector_has_a_valid_length_and_crc() {
        for packet in p2::ALL {
            let (body, tail) = packet.split_at(packet.len() - 2);
            let len = u16::from_le_bytes([packet[5], packet[6]]) as usize;
            assert_eq!(len, packet.len() - 7, "length field of {packet:02X?}");
            assert_eq!(
                crc::update(0, body).to_le_bytes(),
                tail,
                "CRC of {packet:02X?}"
            );
        }
    }

    #[test]
    fn every_p1_vector_has_a_valid_length_and_checksum() {
        for packet in p1::ALL {
            let (body, tail) = packet.split_at(packet.len() - 1);
            assert_eq!(
                packet[3] as usize,
                packet.len() - 4,
                "length of {packet:02X?}"
            );
            let sum = body[2..].iter().fold(0u8, |s, &b| s.wrapping_add(b));
            assert_eq!([!sum], tail, "checksum of {packet:02X?}");
        }
    }
}

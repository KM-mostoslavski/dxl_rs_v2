// C3/C4 for protocol (src/protocol.rs, src/protocol/**): written by /speckit-ov-c4-code.
// Packet facts: https://emanual.robotis.com/docs/en/dxl/protocol1/ and .../protocol2/

!element dxl.lib {
    // C3: components
    exchange = component "Exchange" "Sealed Protocol trait and the crate-internal exchanges: ping, read, write, sync write; P2-only sync read and broadcast ping. Drains RX before TX, bounds every wait by a Clock deadline, decodes every error byte." "Rust traits over P1 | P2 markers" {
        tags "Core"
    }
    p2codec = component "P2 codec" "Streams Protocol 2 instruction packets (byte stuffing, CRC-16) and parses status packets byte by byte with resync, unstuffing and echo skipping." "Rust, src/protocol/p2.rs" {
        tags "Core"
    }
    p1codec = component "P1 codec" "Streams Protocol 1 instruction packets (checksum) and parses status packets byte by byte with resync." "Rust, src/protocol/p1.rs" {
        tags "Core"
    }
    crc = component "CRC-16" "Poly 0x8005, init 0, not reflected; 256-entry table built at compile time." "Rust const fn, src/protocol/crc.rs" {
        tags "Core"
    }
    clock = component "Clock" "Monotonic microsecond time source that bounds every wait; any FnMut() -> u64 closure, StdClock under std." "Rust trait, src/protocol/clock.rs" {
        tags "Core"
    }

    // C4: code elements, grouped by component
    group "Exchange" {
        protocolTrait = component "Protocol trait" "Sealed public trait; BROADCAST = 0xFE." "pub trait" {
            tags "Code"
        }
        exchangeTrait = component "Exchange trait" "ping / read / write / sync_write / report, implemented by P1 and P2." "pub(crate) trait" {
            tags "Code"
        }
        link = component "Link<'a, T, C>" "One wire for one exchange: transport, clock, per-reply timeout." "pub(crate) struct" {
            tags "Code"
        }
        receiveLoop = component "drain / receive / check" "Discard stale RX; poll ReadReady until a Parser completes or the deadline passes; validate ID, error byte, length." "pub(crate) fn" {
            tags "Code"
        }
        writer = component "Writer" "32-byte stack chunk in front of write_all + flush." "pub(crate) struct" {
            tags "Code"
        }
        parserTrait = component "Parser" "Byte-at-a-time status parser: wanted(), push(), on_deadline()." "pub(crate) trait" {
            tags "Code"
        }
    }
    group "P2 codec" {
        p2Type = component "P2" "Protocol 2 marker; inherent sync_read and broadcast_ping." "pub struct" {
            tags "Code"
        }
        p2Parser = component "p2::StatusParser" "Hunt FF FF FD 00, LEN, skip non-status packets, unstuff, check CRC." "pub(crate) struct" {
            tags "Code"
        }
        stuffing = component "Stuffing" "Tracks FF FF FD to add or drop the stuffing 0xFD (instruction + params)." "struct" {
            tags "Code"
        }
    }
    group "P1 codec" {
        p1Type = component "P1" "Protocol 1 marker; no sync read, no broadcast ping (absent by design)." "pub struct" {
            tags "Code"
        }
        p1Parser = component "p1::StatusParser" "Hunt FF FF, LEN, error byte, data, check checksum." "pub(crate) struct" {
            tags "Code"
        }
    }
    group "CRC-16" {
        crcTable = component "crc::update_byte" "Table-driven CRC update over each byte." "const fn" {
            tags "Code"
        }
    }
    group "Clock" {
        clockTrait = component "Clock trait" "now_us(); blanket impl for FnMut() -> u64." "pub trait" {
            tags "Code"
        }
        stdClock = component "StdClock" "Instant-based Clock (std feature)." "pub struct" {
            tags "Code"
        }
    }
}

// C3 relationships
dxl.lib.exchange -> dxl.lib.p2codec "Encodes and parses Protocol 2 packets with"
dxl.lib.exchange -> dxl.lib.p1codec "Encodes and parses Protocol 1 packets with"
dxl.lib.p2codec -> dxl.lib.crc "Checks and appends the CRC with"
dxl.lib.exchange -> dxl.lib.clock "Bounds every wait with"
dxl.lib.exchange -> dxl.lib.transport "Drains, polls ReadReady, reads and writes bytes on" "embedded-io Read + Write + ReadReady"
dxl.lib.exchange -> dxl.lib.errors "Reports transport, protocol and status errors as"
dxl.lib.exchange -> dxl.lib.fault "Hands decoded StatusError (is_hardware_fault) to"
dxl.lib.bus -> dxl.lib.exchange "Is generic over" "P: Protocol"
dxl.lib.bus -> dxl.lib.clock "Owns the time source"
dxl.lib.handle -> dxl.lib.exchange "Sends unicast instructions via"
dxl.lib.multi -> dxl.lib.exchange "Sends sync write / sync read via"
dxl.lib.scan -> dxl.lib.exchange "Pings via"

// C4 relationships
dxl.lib.exchangeTrait -> dxl.lib.protocolTrait "Extends"
dxl.lib.exchangeTrait -> dxl.lib.link "Runs one exchange over"
dxl.lib.exchangeTrait -> dxl.lib.receiveLoop "Uses"
dxl.lib.receiveLoop -> dxl.lib.parserTrait "Feeds bytes to"
dxl.lib.receiveLoop -> dxl.lib.clockTrait "Reads the deadline from"
dxl.lib.p2Type -> dxl.lib.exchangeTrait "Implements"
dxl.lib.p1Type -> dxl.lib.exchangeTrait "Implements"
dxl.lib.p2Type -> dxl.lib.writer "Streams packets through"
dxl.lib.p1Type -> dxl.lib.writer "Streams packets through"
dxl.lib.p2Parser -> dxl.lib.parserTrait "Implements"
dxl.lib.p1Parser -> dxl.lib.parserTrait "Implements"
dxl.lib.p2Type -> dxl.lib.stuffing "Stuffs instruction + params with"
dxl.lib.p2Parser -> dxl.lib.stuffing "Unstuffs with"
dxl.lib.p2Parser -> dxl.lib.crcTable "Checks CRC with"
dxl.lib.stdClock -> dxl.lib.clockTrait "Implements"

// C3/C4 for protocol.
// Source: src/protocol.rs, src/protocol/{clock,crc,p1,p2,vectors,no_alloc}.rs
// Packet facts: https://emanual.robotis.com/docs/en/dxl/protocol1/ and .../protocol2/

!element dxl.lib {
    // C3: components
    protocol_contract = component "protocol: Protocol contract" "Sealed public Protocol trait (BROADCAST, VERSION) with the Exchange supertrait: ping, read, write, sync_write, scan, read_many, report. Link carries one wire (transport, clock, per-reply timeout) into an exchange; Pong is a Ping answer. Exchange, Link and Pong are pub only to be a supertrait of Protocol: the module is private, so they cannot be named outside the crate." "Rust traits, sealed"
    protocol_engine = component "protocol: Exchange engine" "Shared send/receive rules: drain stale RX before every TX, receive byte by byte through a Parser until a packet completes or the Clock deadline passes, check ID / error byte / length, stream TX through a 32-byte stack Writer. Allocates nothing." "Rust, src/protocol.rs"
    protocol_p1 = component "protocol: Protocol 1" "P1 marker and its Exchange impl: checksum-framed instruction packets, byte-by-byte status parser with resync, 1-byte addresses, per-ID scan of 0..=252, read_many as sequential unicast reads. No Sync Read, no broadcast Ping (absent by design)." "Rust, src/protocol/p1.rs"
    protocol_p2 = component "protocol: Protocol 2" "P2 marker and its Exchange impl: byte stuffing and CRC-16 on encode, status parser with resync, unstuffing and echo skipping, one broadcast Ping for scan, read_many as one Sync Read. Inherent P2::sync_read, so it exists for P2 only." "Rust, src/protocol/p2.rs"
    protocol_crc = component "protocol: CRC-16" "Poly 0x8005, init 0, not reflected, no final XOR; 256-entry table built by a const fn at compile time." "Rust const fn, src/protocol/crc.rs"
    protocol_clock = component "protocol: Clock" "Monotonic microsecond time source bounding every wait: any FnMut() -> u64 closure, or StdClock under the std feature." "Rust trait, src/protocol/clock.rs"
    protocol_oracles = component "protocol: Test oracles" "cfg(test) only: e-Manual packet vectors for P1 and P2, a counting global allocator proving exchanges never allocate, and seeded fuzz tests (1M parser inputs, every exchange returns by its deadline)." "Rust, cfg(test)"

    // C3 relationships between own components
    protocol_contract -> protocol_p1 "Is implemented by" "impl Exchange + Protocol for P1"
    protocol_contract -> protocol_p2 "Is implemented by" "impl Exchange + Protocol for P2"
    protocol_p1 -> protocol_engine "Drains, receives, checks and writes with"
    protocol_p2 -> protocol_engine "Drains, receives, checks and writes with"
    protocol_p2 -> protocol_crc "Computes and verifies packet CRC with"
    protocol_engine -> protocol_clock "Bounds every wait by a deadline from"
    protocol_engine -> protocol_contract "Decodes error bytes via Exchange::report of"
    protocol_oracles -> protocol_p1 "Checks encode, decode and robustness of"
    protocol_oracles -> protocol_p2 "Checks encode, decode and robustness of"
    protocol_oracles -> protocol_crc "Verifies every P2 vector's CRC with"

    // C4: code elements, grouped by C3 component
    group "protocol: Protocol contract" {
        protocol_trait = component "protocol::Protocol" "Sealed, public (re-exported at crate root). BROADCAST = 0xFE, VERSION: ProtocolVersion. Supertraits Sealed + Exchange; Bus is generic over it." "Rust trait" {
            tags "Code"
        }
        protocol_sealed = component "protocol::sealed::Sealed" "Private supertrait; only P1 and P2 implement it, so users cannot add protocols." "Rust trait, private module" {
            tags "Code"
        }
        protocol_version = component "protocol::ProtocolVersion" "P1 | P2 as a value for messages and runtime checks; Display gives \"Protocol 1\" / \"Protocol 2\"." "Rust enum" {
            tags "Code"
        }
        protocol_exchange = component "protocol::Exchange" "The instructions both protocols support: ping, read, write, sync_write (broadcast, never waits), scan, read_many, report. Protocol-specific instructions live on the protocol type instead." "Rust trait, pub in a private module" {
            tags "Code"
        }
        protocol_link = component "protocol::Link<'a, T, C>" "One wire for one exchange: &mut transport, &mut clock, per-reply timeout; deadline() = now_us + timeout (saturating)." "Rust struct" {
            tags "Code"
        }
        protocol_pong = component "protocol::Pong" "Ping answer: Id, model number and firmware (P2 only), fault (scan replies with a set error byte)." "Rust struct" {
            tags "Code"
        }
    }

    group "protocol: Exchange engine" {
        protocol_drain = component "protocol::drain" "Discards pending RX before TX; gives up after one timeout if the line keeps talking; zero-byte read = end of stream." "Rust fn, pub(crate)" {
            tags "Code"
        }
        protocol_receive = component "protocol::receive" "Polls ReadReady and reads only parser.wanted() bytes (max CHUNK) until the parser completes or the deadline passes; deadline yields Timeout or the parser's Malformed." "Rust fn, pub(crate)" {
            tags "Code"
        }
        protocol_check = component "protocol::check" "Validates a parsed Status against the expected Id and out.len(); a set error byte wins over a wrong length and becomes StatusError via P::report." "Rust fn, pub(crate)" {
            tags "Code"
        }
        protocol_parser = component "protocol::Parser" "Byte-at-a-time status parser: wanted() >= 1, push(byte), on_deadline()." "Rust trait, pub(crate)" {
            tags "Code"
        }
        protocol_status = component "protocol::Status" "Parsed, integrity-checked status packet header: raw id, error byte, data length (data already in the caller's buffer)." "Rust struct, pub(crate)" {
            tags "Code"
        }
        protocol_writer = component "protocol::Writer" "32-byte (CHUNK) stack buffer in front of write_all; finish() writes the rest and flushes." "Rust struct, pub(crate)" {
            tags "Code"
        }
        protocol_io_error = component "protocol::io_error" "Maps an embedded-io error to Error::Transport; ErrorKind::TimedOut becomes TransportError::Timeout." "Rust fn, pub(crate)" {
            tags "Code"
        }
    }

    group "protocol: Protocol 1" {
        protocol_p1_type = component "protocol::P1" "Protocol 1.0 marker (re-exported at crate root); implements Protocol and Exchange; report() decodes P1Flags." "Rust unit struct" {
            tags "Code"
        }
        protocol_p1_send = component "protocol::p1::send" "Streams FF FF | ID | LEN | INST | params | ~sum; refuses LEN > 255 before sending." "Rust fn, private" {
            tags "Code"
        }
        protocol_p1_parser = component "protocol::p1::StatusParser" "Hunts FF FF, reads ID, LEN, error byte, data, checksum; resyncs on impossible LEN and reports Malformed at the deadline." "Rust struct, pub(crate)" {
            tags "Code"
        }
        protocol_p1_unicast = component "protocol::p1::unicast" "drain, send, receive until the expected ID answers (late replies from other IDs dropped), check." "Rust fn, private" {
            tags "Code"
        }
    }

    group "protocol: Protocol 2" {
        protocol_p2_type = component "protocol::P2" "Protocol 2.0 marker (re-exported at crate root); implements Protocol and Exchange; report() decodes alert bit 7 + P2Code." "Rust unit struct" {
            tags "Code"
        }
        protocol_p2_send = component "protocol::p2::send" "Streams FF FF FD 00 | ID | LEN | INST | params | CRC with stuffing from the instruction byte on; refuses LEN > 65535 before sending." "Rust fn, pub(super)" {
            tags "Code"
        }
        protocol_p2_stuffing = component "protocol::p2::Stuffing" "Tracks the FF FF FD pattern to add (encode) or drop (decode) the stuffing 0xFD." "Rust struct, private" {
            tags "Code"
        }
        protocol_p2_parser = component "protocol::p2::StatusParser" "Hunts FF FF FD 00, skips non-status (echoed instruction) packets, bounds LEN by out.len(), unstuffs, checks CRC." "Rust struct, pub(crate)" {
            tags "Code"
        }
        protocol_p2_unicast = component "protocol::p2::unicast" "drain, send, receive until the expected ID answers (late replies from other IDs dropped), check." "Rust fn, private" {
            tags "Code"
        }
        protocol_p2_sync_read = component "protocol::P2::sync_read" "One Sync Read; on_reply once per ID in request order, full timeout per reply; a reply from a later ID times out the earlier ones; late replies from timed-out IDs discarded; Err only on transport failure or invalid request." "Rust inherent fn, pub(crate)" {
            tags "Code"
        }
        protocol_p2_scan_reply = component "protocol::p2::scan_reply" "Turns one broadcast-Ping reply into a Pong (fault kept) or the error explaining why it cannot be one." "Rust fn, private" {
            tags "Code"
        }
    }

    group "protocol: CRC-16" {
        protocol_crc_table = component "protocol::crc::TABLE" "[u16; 256] built at compile time by const fn table() from POLY 0x8005." "Rust const" {
            tags "Code"
        }
        protocol_crc_update_byte = component "protocol::crc::update_byte" "Table-driven CRC update for one byte; crc::update folds it over a slice." "Rust const fn, pub(crate)" {
            tags "Code"
        }
    }

    group "protocol: Clock" {
        protocol_clock_trait = component "protocol::Clock" "now_us(&mut self) -> u64, never going backwards; public, re-exported at crate root; on_unimplemented diagnostic." "Rust trait" {
            tags "Code"
        }
        protocol_clock_closure = component "protocol::clock::impl Clock for FnMut() -> u64" "Blanket impl: any microsecond closure is a Clock (one-line embedded setup)." "Rust blanket impl" {
            tags "Code"
        }
        protocol_std_clock = component "protocol::StdClock" "Clock backed by std::time::Instant; cfg(feature = \"std\")." "Rust struct, std feature" {
            tags "Code"
        }
    }

    group "protocol: Test oracles" {
        protocol_vectors = component "protocol::vectors" "cfg(test): e-Manual packet vectors (p1, p2 submodules) fetched 2026-10-26, each self-checked for LEN and checksum/CRC." "Rust module, cfg(test)" {
            tags "Code"
        }
        protocol_no_alloc = component "protocol::no_alloc" "cfg(test): per-thread counting GlobalAlloc; exchanges_never_allocate runs every P1/P2 exchange against MockTransport." "Rust module, cfg(test)" {
            tags "Code"
        }
        protocol_robustness = component "protocol::tests::robustness" "cfg(test): seeded XorShift fuzz (DXL_FUZZ_SEED): 1M random inputs through both parsers without panic; every exchange returns by its deadline." "Rust test module, cfg(test)" {
            tags "Code"
        }
    }

    // C4 relationships
    protocol_trait -> protocol_sealed "Requires (sealed)"
    protocol_trait -> protocol_exchange "Requires as supertrait"
    protocol_trait -> protocol_version "Exposes as VERSION"
    protocol_exchange -> protocol_link "Runs each exchange over"
    protocol_exchange -> protocol_pong "Returns from ping and scan"
    protocol_link -> protocol_clock_trait "Computes deadlines with"
    protocol_receive -> protocol_parser "Feeds bytes to"
    protocol_receive -> protocol_status "Returns"
    protocol_receive -> protocol_io_error "Maps transport errors with"
    protocol_drain -> protocol_io_error "Maps transport errors with"
    protocol_writer -> protocol_io_error "Maps transport errors with"
    protocol_check -> protocol_exchange "Decodes the error byte with" "P::report"
    protocol_p1_type -> protocol_exchange "Implements"
    protocol_p1_type -> protocol_trait "Implements"
    protocol_p1_type -> protocol_p1_unicast "Runs ping, read and write through"
    protocol_p1_type -> protocol_p1_send "Sends sync write with"
    protocol_p1_unicast -> protocol_drain "Clears RX with"
    protocol_p1_unicast -> protocol_p1_send "Sends the instruction with"
    protocol_p1_unicast -> protocol_receive "Waits for the reply with"
    protocol_p1_unicast -> protocol_check "Validates the reply with"
    protocol_p1_send -> protocol_writer "Streams bytes through"
    protocol_p1_parser -> protocol_parser "Implements"
    protocol_p1_unicast -> protocol_p1_parser "Parses the reply with"
    protocol_p2_type -> protocol_exchange "Implements"
    protocol_p2_type -> protocol_trait "Implements"
    protocol_p2_type -> protocol_p2_unicast "Runs ping, read and write through"
    protocol_p2_type -> protocol_p2_sync_read "Implements read_many as"
    protocol_p2_type -> protocol_p2_scan_reply "Classifies broadcast-Ping replies with"
    protocol_p2_unicast -> protocol_drain "Clears RX with"
    protocol_p2_unicast -> protocol_p2_send "Sends the instruction with"
    protocol_p2_unicast -> protocol_receive "Waits for the reply with"
    protocol_p2_unicast -> protocol_check "Validates the reply with"
    protocol_p2_unicast -> protocol_p2_parser "Parses the reply with"
    protocol_p2_sync_read -> protocol_p2_send "Sends one Sync Read with"
    protocol_p2_sync_read -> protocol_receive "Waits for each reply with"
    protocol_p2_sync_read -> protocol_check "Validates each reply with"
    protocol_p2_send -> protocol_writer "Streams bytes through"
    protocol_p2_send -> protocol_p2_stuffing "Stuffs instruction and params with"
    protocol_p2_send -> protocol_crc_update_byte "Accumulates the CRC with"
    protocol_p2_parser -> protocol_parser "Implements"
    protocol_p2_parser -> protocol_p2_stuffing "Unstuffs with"
    protocol_p2_parser -> protocol_crc_update_byte "Verifies the CRC with"
    protocol_crc_update_byte -> protocol_crc_table "Looks up"
    protocol_clock_closure -> protocol_clock_trait "Implements"
    protocol_std_clock -> protocol_clock_trait "Implements"
    protocol_vectors -> protocol_crc_update_byte "Self-checks P2 vectors with"
    protocol_no_alloc -> protocol_exchange "Calls every exchange of P1 and P2 under a counting allocator"
    protocol_robustness -> protocol_p1_parser "Fuzzes"
    protocol_robustness -> protocol_p2_parser "Fuzzes"
}

// Relationships with workspace.dsl elements (full hierarchical ids)
dxl.lib.bus -> dxl.lib.protocol_contract "Is generic over P: Protocol; runs every exchange through bus::wire" "P: Protocol (compile time)"
dxl.lib.bus -> dxl.lib.protocol_clock "Owns the time source C: Clock (DefaultClock under std)"
dxl.lib.multi -> dxl.lib.protocol_p2 "Calls P2::sync_read for the explicit tier on Bus<P2, T>"
dxl.lib.scan -> dxl.lib.protocol_contract "Discovers actuators with Exchange::scan and ping"
dxl.lib.fault -> dxl.lib.protocol_contract "Classifies the StatusError decoded by Exchange::report"
dxl.lib.protocol_engine -> dxl.lib.transport "Drains, polls ReadReady, reads and writes bytes on" "embedded-io Read + Write + ReadReady"
dxl.lib.protocol_engine -> dxl.lib.errors "Reports Transport, Protocol and Status errors as"
dxl.lib.protocol_p1 -> dxl.lib.errors "Decodes the P1 error byte into StatusReport::P1(P1Flags); refuses bad requests as ValidationError"
dxl.lib.protocol_p2 -> dxl.lib.errors "Decodes the P2 error byte into StatusReport::P2 { alert, code: P2Code }; refuses bad requests as ValidationError"
dxl.lib.protocol_contract -> dxl.lib.units "Addresses actuators by" "Id"
dxl.lib.protocol_p1 -> actuators "Exchanges instruction and status packets with" "Dynamixel Protocol 1.0"
dxl.lib.protocol_p2 -> actuators "Exchanges instruction and status packets with" "Dynamixel Protocol 2.0"
dxl.lib.protocol_oracles -> emanual "Copies packet examples from" "HTTPS, fetched 2026-10-26"
dxl.lib.protocol_oracles -> dxl.lib.transport "Scripts exchanges with" "MockTransport + MockTime"

// C4 relationships with workspace.dsl elements
dxl.lib.protocol_exchange -> dxl.lib.units "Takes and returns" "Id"
dxl.lib.protocol_link -> dxl.lib.transport "Borrows" "&mut T: Transport"
dxl.lib.protocol_check -> dxl.lib.errors "Returns WrongId, StatusError or WrongLength as"
dxl.lib.protocol_io_error -> dxl.lib.errors "Produces" "TransportError"

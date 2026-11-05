// C3/C4 for transport.
// Code: src/transport.rs, src/transport/serial.rs (feature std), src/transport/mock.rs (feature mock, or crate tests).
// The C2 component dxl.lib.transport is the module as a whole; this file adds what is inside it.

!element dxl.lib {

    // C3: components

    transport_contract = component "transport: Contract" "Transport = embedded_io Read + Write + ReadReady with a blanket impl, so any UART with the three traits is a transport, no_std and heap-free. Knows no packets and keeps no time: the bus polls read_ready against its Clock. Documented rules: read_ready never blocks, read after Ok(true) returns at least one byte (Ok(0) = end of stream), flush returns once bytes left (DE/RE back to receive), no echo, no internal retry, a timeout is ErrorKind::TimedOut. Re-exports adapters by feature." "Rust, embedded-io 0.7 (blocking), src/transport.rs"

    transport_serial = component "transport: Serial adapter" "SerialTransport: an OS serial port (U2D2, USB-TTL, RS-485) as a Transport. open(path, baud) sets 8N1, no flow control, 50 ms port timeout; From wraps a port the caller opened, keeping its settings. read_ready via bytes_to_read; a write that accepts nothing becomes TimedOut. Error = std::io::Error. Never changes OS driver settings (FTDI latency timer is documented only). Feature std." "Rust, serialport 4.9, src/transport/serial.rs"

    transport_mock_line = component "transport: Mock line" "MockTransport: a scripted, allocation-free serial line for tests without hardware. Fixed-array capacities as const generics (RX, TX, CHUNKS, defaults 4096/4096/32); records every sent byte, counts reads, writes and flushes; Error = embedded_io::ErrorKind. Public with feature mock, pub(crate) in crate tests." "Rust, src/transport/mock.rs"

    transport_mock_script = component "transport: Mock script" "Incoming bytes scripted as chunks, each with a gate: stale bytes open from the start, replies open after the n-th flush and once mock time reaches t. Chunks open strictly in script order. Fault injection: fail_with(kind) fails every later call, end_of_stream makes the line ready but read 0 bytes." "Rust, src/transport/mock.rs"

    transport_mock_time = component "transport: Mock time" "MockTime: shared monotonic test time in microseconds (Cell<u64>), read by the mock's gates; ticker(step) returns an FnMut() -> u64 that the bus accepts as its Clock." "Rust, src/transport/mock.rs"

    transport_mock_checks = component "transport: Sent-byte checks" "Test assertions on a MockTransport: check_sent returns the first Mismatch (offset, expected and actual byte), assert_sent panics with both byte strings in hex, assert_done panics when scripted bytes were never read." "Rust, src/transport/mock.rs"

    transport_serial -> transport_contract "Implements" "embedded_io Read + Write + ReadReady"
    transport_mock_line -> transport_contract "Implements" "embedded_io Read + Write + ReadReady"
    transport_mock_line -> transport_mock_script "Releases incoming bytes by evaluating"
    transport_mock_script -> transport_mock_time "Gates replies on"
    transport_mock_checks -> transport_mock_line "Compares the recorded sent bytes of"

    // C4: code elements, grouped by C3 component

    group "transport: Contract" {
        transport_mod = component "transport" "Private module: declares serial behind cfg(feature = \"std\") and mock behind cfg(any(test, feature = \"mock\")); re-exports SerialTransport (std), MockTransport, MockTime and Mismatch (mock) to the crate root." "Rust module" {
            tags "Code"
        }
        transport_trait = component "transport::Transport" "pub trait Transport: Read + Write + ReadReady {}: no methods of its own. A type with only Read + Write is rejected at compile time (E0277), since the bus could not bound its waits." "Rust trait" {
            tags "Code"
        }
        transport_blanket_impl = component "impl<T: Read + Write + ReadReady> Transport for T" "Blanket impl: nothing to implement by hand and no wrapper to write for a HAL UART." "Rust blanket impl" {
            tags "Code"
        }
    }

    group "transport: Serial adapter" {
        transport_serial_transport = component "transport::serial::SerialTransport" "pub struct owning a NativePort; Debug. port_mut and into_inner give the OS port back for settings it does not cover; From<NativePort> wraps an already open port." "Rust struct" {
            tags "Code"
        }
        transport_serial_open = component "transport::serial::SerialTransport::open" "open(path: &str, baud_rate: u32) -> serialport::Result<Self>: serialport::new with 8 data bits, no parity, 1 stop bit, no flow control, PORT_TIMEOUT, then open_native. On Unix a missing port is Io(NotFound), a held port NoDevice." "Rust fn" {
            tags "Code"
        }
        transport_serial_native_port = component "transport::serial::NativePort" "Type alias: serialport::TTYPort on Unix, serialport::COMPort on Windows; no dyn, no Box." "Rust type alias, cfg(unix) / cfg(windows)" {
            tags "Code"
        }
        transport_serial_port_timeout = component "transport::serial::PORT_TIMEOUT" "50 ms. Bounds a read that races an empty buffer, the wait for the OS to accept written bytes, and the retry window of an interrupted flush on Unix. Never delays a reply: the bus reads only after read_ready." "Rust const Duration" {
            tags "Code"
        }
        transport_serial_io_impls = component "transport::serial::SerialTransport: embedded_io impls" "ErrorType (Error = std::io::Error), Read and Write delegate to std::io on the port, ReadReady = bytes_to_read() > 0." "Rust trait impls" {
            tags "Code"
        }
        transport_serial_accepted = component "transport::serial::accepted" "Maps a write that accepted 0 bytes of a non-empty buffer to io::ErrorKind::TimedOut: a Windows write timeout is Ok(0), which embedded-io forbids (write_all would panic)." "Rust fn, private" {
            tags "Code"
        }
    }

    group "transport: Mock line" {
        transport_mock_transport = component "transport::mock::MockTransport" "pub struct MockTransport<'t, const RX = 4096, const TX = 4096, const CHUNKS = 32>: rx/tx byte arrays, chunk table, counters, injected fault, eof flag, borrowed &MockTime. new(&time) for default capacities, with_capacity(&time) for others. Debug shows counters and sent bytes, not the raw buffers." "Rust struct, const generics" {
            tags "Code"
        }
        transport_mock_io_impls = component "transport::mock::MockTransport: embedded_io impls" "ErrorType (Error = ErrorKind). read_ready: injected fault, else eof or open bytes waiting. read copies only open bytes. write records up to TX, WriteZero when full. flush counts one sent packet. Every call fails first if fail_with was set." "Rust trait impls" {
            tags "Code"
        }
        transport_mock_counters = component "transport::mock::MockTransport::sent" "With reads, writes, flushes and unread: every byte written so far and the call counters, for tests that assert on traffic." "Rust fns" {
            tags "Code"
        }
    }

    group "transport: Mock script" {
        transport_mock_gate = component "transport::mock::Gate" "Private enum: Always (stale bytes left on the line) or AfterTx { flushes, at } (open once that many packets were flushed and time reached at)." "Rust enum, private" {
            tags "Code"
        }
        transport_mock_stale = component "transport::mock::MockTransport::stale" "Scripts bytes already waiting before anything is sent (Gate::Always). Panics past RX bytes or CHUNKS chunks." "Rust fn, builder" {
            tags "Code"
        }
        transport_mock_reply_at = component "transport::mock::MockTransport::reply_at" "Scripts bytes that arrive after the next flushed packet once time reaches at_us; reply(bytes) is reply_at(0, bytes). Lets a test script each reply right before its exchange." "Rust fn, builder" {
            tags "Code"
        }
        transport_mock_reply_to = component "transport::mock::MockTransport::reply_to" "Scripts bytes that arrive after the n-th flush since creation, to script several exchanges up front." "Rust fn, builder" {
            tags "Code"
        }
        transport_mock_fail_with = component "transport::mock::MockTransport::fail_with" "Injects an embedded_io::ErrorKind returned by every later read, write, flush and read_ready; end_of_stream() instead reports ready but reads 0 bytes, like a disconnected port." "Rust fn, builder" {
            tags "Code"
        }
        transport_mock_available_end = component "transport::mock::MockTransport::available_end" "Walks the chunk table in script order and stops at the first closed gate, so a chunk not readable yet hides every chunk after it." "Rust fn, private" {
            tags "Code"
        }
    }

    group "transport: Mock time" {
        transport_mock_time_struct = component "transport::mock::MockTime" "pub struct over Cell<u64> µs: new() (const, time zero), now, set (backwards ignored, monotonic), advance (saturating)." "Rust struct" {
            tags "Code"
        }
        transport_mock_ticker = component "transport::mock::MockTime::ticker" "ticker(step_us) -> impl FnMut() -> u64: advances time by step_us on every call and returns it, usable as the bus Clock." "Rust fn" {
            tags "Code"
        }
    }

    group "transport: Sent-byte checks" {
        transport_mock_mismatch = component "transport::mock::Mismatch" "pub struct { offset, expected: Option<u8>, actual: Option<u8> }: where sent bytes first differ from the expected ones; Display for assertion messages. Public with feature mock only." "Rust struct" {
            tags "Code"
        }
        transport_mock_check_sent = component "transport::mock::MockTransport::check_sent" "check_sent(expected) -> Result<(), Mismatch>: the first differing offset, or the shorter length when one is a prefix of the other." "Rust fn" {
            tags "Code"
        }
        transport_mock_assert_sent = component "transport::mock::MockTransport::assert_sent" "Panics (track_caller) on the first Mismatch, printing sent and expected bytes in hex." "Rust fn" {
            tags "Code"
        }
        transport_mock_assert_done = component "transport::mock::MockTransport::assert_done" "Panics (track_caller) when scripted bytes were left unread." "Rust fn" {
            tags "Code"
        }
    }

    // C4 relationships
    transport_mod -> transport_trait "Declares"
    transport_blanket_impl -> transport_trait "Implements for every Read + Write + ReadReady type"
    transport_serial_transport -> transport_serial_native_port "Owns"
    transport_serial_open -> transport_serial_port_timeout "Sets the port timeout to"
    transport_serial_open -> transport_serial_transport "Builds"
    transport_serial_io_impls -> transport_serial_transport "Are implemented on"
    transport_serial_io_impls -> transport_serial_accepted "Checks every write with"
    transport_serial_transport -> transport_blanket_impl "Is a Transport through"
    transport_mock_transport -> transport_blanket_impl "Is a Transport through"
    transport_mock_io_impls -> transport_mock_transport "Are implemented on"
    transport_mock_io_impls -> transport_mock_available_end "Read only bytes up to"
    transport_mock_counters -> transport_mock_transport "Reports the traffic of"
    transport_mock_transport -> transport_mock_time_struct "Borrows" "&'t MockTime"
    transport_mock_stale -> transport_mock_gate "Pushes a chunk with" "Gate::Always"
    transport_mock_reply_at -> transport_mock_gate "Pushes a chunk with" "Gate::AfterTx"
    transport_mock_reply_to -> transport_mock_gate "Pushes a chunk with" "Gate::AfterTx"
    transport_mock_fail_with -> transport_mock_io_impls "Makes every call of fail"
    transport_mock_available_end -> transport_mock_gate "Evaluates"
    transport_mock_available_end -> transport_mock_time_struct "Reads now from"
    transport_mock_ticker -> transport_mock_time_struct "Advances"
    transport_mock_check_sent -> transport_mock_counters "Reads sent bytes from"
    transport_mock_check_sent -> transport_mock_mismatch "Returns"
    transport_mock_assert_sent -> transport_mock_check_sent "Panics on the result of"
    transport_mock_assert_done -> transport_mock_counters "Checks unread from"
}

// C3 relationships to workspace elements
dxl.lib.transport_contract -> dxl.lib.transport "Is the contract of"
dxl.lib.bus -> dxl.lib.transport_contract "Is generic over; polls read_ready against its Clock and reads only waiting bytes" "T: Transport"
dxl.lib.transport_contract -> dxl.lib.errors "Errors surface as T::Error kinds; protocol::io_error maps TimedOut to TransportError::Timeout, others to TransportError::Io" "embedded_io::Error::kind"
dxl.lib.transport_serial -> port "Opens and drives" "serialport open_native, 8N1"
dxl.lib.transport_mock_time -> dxl.lib.bus "Serves as the test Clock of" "FnMut() -> u64"

// C3/C4 for transport (src/transport.rs, src/transport/**): written by /speckit-ov-c4-code.
// The C2-level "Transport" component (dxl.lib.transport) is the contract; this file adds the
// adapters behind it and their code elements.

!element dxl.lib {
    // C3: components
    serialAdapter = component "Serial adapter" "OS serial port as a Transport: opens 8N1 by path and baud, or wraps a port the user opened; ReadReady via bytes_to_read; 50 ms port timeout bounds read races, write acceptance and flush retries; a write that accepts nothing is TimedOut. std feature only." "Rust, serialport 4.9, src/transport/serial.rs" {
        tags "Core"
    }
    mockLine = component "Mock transport" "Scripted, allocation-free line for tests without hardware: gated replies, injected failures, sent-byte checks. mock feature (and crate tests)." "Rust, src/transport/mock.rs" {
        tags "Core"
    }

    // C4: code elements, grouped by component
    group "Transport" {
        transportTrait = component "Transport trait" "Marker trait = embedded_io Read + Write + ReadReady, blanket impl for every such type; no methods, no clock, no retries." "pub trait, src/transport.rs" {
            tags "Code"
        }
    }
    group "Serial adapter" {
        serialTransport = component "SerialTransport" "open(path, baud), From<TTYPort | COMPort>, port_mut, into_inner; Error = std::io::Error." "pub struct" {
            tags "Code"
        }
        nativePort = component "NativePort" "serialport::TTYPort (Unix) or COMPort (Windows), from open_native(): no dyn, no Box." "private type alias" {
            tags "Code"
        }
    }
    group "Mock transport" {
        mockTransport = component "MockTransport<'t, RX, TX, CHUNKS>" "Fixed-array incoming script and outgoing record; stale / reply / reply_at / reply_to / fail_with / end_of_stream; WriteZero when full; Error = embedded_io::ErrorKind." "pub struct, const generics" {
            tags "Code"
        }
        gate = component "Gate" "When a scripted chunk opens: Always, or after the n-th flush once time reaches t; chunks open strictly in script order." "private enum" {
            tags "Code"
        }
        mockTime = component "MockTime" "Shared monotonic test time (Cell<u64> µs); ticker(step) is an FnMut() -> u64 usable as the bus clock." "pub struct" {
            tags "Code"
        }
        mismatch = component "Mismatch" "First differing offset, expected and actual byte, from check_sent; Display for assert_sent." "pub struct" {
            tags "Code"
        }
    }
}

// C3 relationships
dxl.lib.serialAdapter -> dxl.lib.transport "Implements"
dxl.lib.mockLine -> dxl.lib.transport "Implements"
dxl.lib.serialAdapter -> port "Opens and drives" "serialport open_native"
dxl.lib.serialAdapter -> dxl.lib.errors "Reports OS errors by kind to" "std::io::Error -> embedded_io::ErrorKind"

// C4 relationships
dxl.lib.serialTransport -> dxl.lib.transportTrait "Implements"
dxl.lib.serialTransport -> dxl.lib.nativePort "Owns"
dxl.lib.mockTransport -> dxl.lib.transportTrait "Implements"
dxl.lib.mockTransport -> dxl.lib.gate "Opens scripted chunks by"
dxl.lib.mockTransport -> dxl.lib.mockTime "Reads time from"
dxl.lib.mockTransport -> dxl.lib.mismatch "Reports sent-byte differences as"

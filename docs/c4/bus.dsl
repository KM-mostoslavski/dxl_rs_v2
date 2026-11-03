// C3/C4 for bus: written by /speckit-ov-c4-code
// Source: src/bus.rs, src/bus/** (integration worktree after core-3); spec: docs/srs/modules/bus.md.
// The C2 components dxl.lib.bus, handle, multi, scan and fault are realized by the C3 components below.

!element dxl.lib {

    // C3: components

    bus_core = component "bus: Bus and storage" "Bus<P, T, C = DefaultClock, const N = 32>: owns the transport, the clock, a Config (timeout, scan_timeout, retries) and up to N actuators in heapless::Vec<ActuatorKind, N> plus one TorqueState per slot; N <= 253 checked at compile time. Lends one ActuatorRef at a time; SerialBus opens a serial port under std. Storage is reached through the Slots trait so the engine is storage-agnostic." "Rust, src/bus.rs, src/bus/slots.rs, src/bus/serial.rs"
    bus_dyn = component "bus: DynBus" "Bus with no capacity limit (std): alloc::Vec storage grown at Setup only, same methods as Bus, run through the same setup, multi and group engine." "Rust, std feature, src/bus/dyn_bus.rs"
    bus_handle = component "bus: ActuatorRef" "Short-lived handle borrowing one slot, its torque state and the wire: torque on/off, goal position (typed and raw) checked against torque state and the cached limit mode before any byte, goal/present position reads, clear_fault. Raw register read/write by address behind raw-registers." "Rust, src/bus/actuator_ref.rs, src/bus/raw.rs"
    bus_setup = component "bus: Setup" "declare (ping each listed Id, check model number) and autoscan (P2 broadcast ping, P1 sweep 0..=252): instantiate ActuatorKind per model and firmware, read EEPROM limits and Operating Mode (or derive it from angle limits), push with torque Off; any failure clears all slots." "Rust, src/bus/setup.rs"
    bus_multi = component "bus: Multi-actuator API" "Easy tier on every Bus<P, T, C, N> (enable_torque, disable_torque, write_goal_positions[_raw], read_positions[_raw]), explicit sync_write (P1 + P2) and sync_read (P2 only, absent on P1), partial reads behind partial-reads; targets are any IntoIterator<Item = Id> (IdSet included); values typed by sealed field markers; reads fill caller-owned buffers." "Rust, src/bus/multi.rs, src/bus/partial.rs, src/bus/field.rs, src/bus/id_set.rs"
    bus_group = component "bus: Group engine" "N-free core over slices: resolve targets to slots, check value/buffer counts, validate and encode every target before any byte, group stages by (addr, size) and send one instruction per group; keeps the first error, preferring a hardware fault." "Rust, src/bus/group.rs"
    bus_observe = component "bus: Fault observer" "The one place replies move a TorqueState: a status reply reporting a hardware fault marks the slot Faulted; a Torque Enable write sets On/Off, Faulted or Off on failure; clear_fault reads Hardware Error Status (or pings) and clears only on a clean answer." "Rust, src/bus/observe.rs"
    bus_wire = component "bus: Wire" "The bus's only door to the protocol: borrows transport, clock and Config per call, builds a protocol Link, retries unicast exchanges up to Config::retries on timeout or corrupted reply only; scan, sync write and multi-reads are never retried; encodes/decodes register values." "Rust, src/bus/wire.rs"

    // C4: code elements

    group "bus: Bus and storage" {
        bus_bus_struct = component "bus::Bus" "Bus<P: Protocol, T: Transport, C = DefaultClock, const N: usize = 32>: new, into_parts, config, declare, autoscan, actuators, ids, actuator(id) -> Option<ActuatorRef>, torque_state(s)." "Rust struct" {
            tags "Code"
        }
        bus_config = component "bus::Config" "timeout (10 ms), scan_timeout (10 ms), retries (0); const builder methods; non_exhaustive." "Rust struct" {
            tags "Code"
        }
        bus_default_clock = component "bus::DefaultClock" "StdClock under std; NoClock (uninhabited enum) without std, so a no_std bus must name its clock." "Rust type alias" {
            tags "Code"
        }
        bus_serial_bus = component "bus::SerialBus" "Bus<P, SerialTransport, StdClock, N> with open(path, baud); sends nothing until Setup." "Rust type alias + impl, std feature" {
            tags "Code"
        }
        bus_slots = component "bus::slots::Slots" "kinds, split_mut, clear, push (TooManyActuators past capacity); actuators kept ascending by Id." "Rust trait, pub(crate)" {
            tags "Code"
        }
        bus_fixed_slots = component "bus::slots::FixedSlots" "Slots over a Bus's heapless::Vec<ActuatorKind, N> and [TorqueState; N]; states past len stay Off." "Rust struct, pub(crate)" {
            tags "Code"
        }
        bus_find = component "bus::slots::find" "Binary search of an Id in the sorted actuator list." "Rust fn, pub(crate)" {
            tags "Code"
        }
    }

    group "bus: DynBus" {
        bus_dynbus_struct = component "bus::DynBus" "DynBus<P, T, C = StdClock>: Bus API without N (setup, actuator, Easy tier, sync_write, sync_read on P2, partial reads behind partial-reads)." "Rust struct, std feature" {
            tags "Code"
        }
        bus_dyn_slots = component "bus::dyn_bus::DynSlots" "Slots over two alloc::Vec (actuators, torque states) that grow at Setup only." "Rust struct, private" {
            tags "Code"
        }
    }

    group "bus: ActuatorRef" {
        bus_actuator_ref = component "bus::ActuatorRef" "ActuatorRef<'b, P, T, C>: kind, torque_state, enable_torque, disable_torque, set_goal_position[_raw], goal_position[_raw], present_position[_raw], clear_fault; implements Describe and Actuator." "Rust struct" {
            tags "Code"
        }
        bus_raw = component "bus::raw" "ActuatorRef::read_raw(addr, out) and write_raw(addr, data): bypass torque guard, limits and cache; replies still pass the fault check." "Rust impl block, raw-registers feature" {
            tags "Code"
        }
    }

    group "bus: Setup" {
        bus_declare = component "bus::setup::declare" "Rejects duplicate Ids and unsupported models before any byte; pings each Id in ascending order (timeout -> SetupError::Missing), reads Model Number if the pong lacks it (P1), WrongModel on mismatch." "Rust fn, pub(crate)" {
            tags "Code"
        }
        bus_autoscan = component "bus::setup::autoscan" "Collects pongs via Wire::scan (fault in a pong, duplicate Id or more than N fails), sorts by Id, then initializes each." "Rust fn, pub(crate)" {
            tags "Code"
        }
        bus_init_one = component "bus::setup::init_one" "Firmware from pong or register, ActuatorKind::new, read min/max limits into the cache, read Operating Mode into a LimitMode (UnknownOperatingMode otherwise) or LimitMode::from_angle_limits, push with torque Off." "Rust fn, private" {
            tags "Code"
        }
    }

    group "bus: Multi-actuator API" {
        bus_multi_easy = component "bus::multi" "impl Bus<P, T, C, N>: enable_torque / disable_torque (acknowledged unicast writes per target), write_goal_positions[_raw] and sync_write (Sync Write per layout group), read_positions[_raw] (Sync Read on P2, one Read per Id on P1), all-or-nothing validation." "Rust impl block + generic helpers torque_on, torque_off, write_field, read_field" {
            tags "Code"
        }
        bus_sync_read = component "bus::Bus::sync_read" "Explicit Sync Read of a ReadField into a caller-owned buffer; only on impl Bus<P2, T, C, N>, so a P1 call does not compile." "Rust method" {
            tags "Code"
        }
        bus_partial = component "bus::partial" "read_positions_partial / read_positions_raw_partial: out[i] is Result per target; only refusals fail the call; a failed group instruction is shared by its group (OnFailedGroup::Share)." "Rust impl block, partial-reads feature" {
            tags "Code"
        }
        bus_field_traits = component "bus::field::ReadField / WriteField" "Public field traits with an associated Value type (Position or i32), built on the sealed Field (register in ModelData) and Value (to_raw / from_raw via PositionScale) traits." "Rust traits, sealed" {
            tags "Code"
        }
        bus_field_markers = component "bus::field::GoalPosition / PresentPosition / Raw" "Field markers passed as values: GoalPosition (write, Position), PresentPosition (read, Position), Raw<F> (same register, i32 ticks). Torque Enable is deliberately not a field." "Rust unit structs" {
            tags "Code"
        }
        bus_id_set = component "bus::IdSet" "Copy 32-byte bitset of Ids, ascending iteration via IdSetIter; range, insert, remove, contains, len; FromIterator, Extend, From<[Id; K]>." "Rust struct" {
            tags "Code"
        }
    }

    group "bus: Group engine" {
        bus_stage = component "bus::group::Stage" "8-byte per-target record: slot, register addr and size, up to 4 encoded or received bytes; key() = (addr, size)." "Rust struct, pub(crate)" {
            tags "Code"
        }
        bus_group_validate = component "bus::group::resolve / validate_writes" "resolve maps targets to slots (NotOnBus, DuplicateId); check_values / check_out guard counts; validate_writes checks torque state and limit mode and encodes each target with its own register layout." "Rust fns, pub(crate)" {
            tags "Code"
        }
        bus_group_groups = component "bus::group::groups / entries" "Distinct (addr, size) keys in first-appearance order, and the (Id, bytes) entries of one group for a Sync Write." "Rust fns, pub(crate)" {
            tags "Code"
        }
        bus_read_each = component "bus::group::read_each" "Runs one ReadMany per layout group and hands each target its raw value or error; OnFailedGroup::Abort, or Share under partial-reads. read_stages wraps it for strict reads." "Rust fn, pub(crate)" {
            tags "Code"
        }
        bus_first_error = component "bus::group::FirstError" "Keeps the error to return: a hardware fault beats any other error, else the earliest target wins." "Rust struct, pub(crate)" {
            tags "Code"
        }
    }

    group "bus: Fault observer" {
        bus_observe_fn = component "bus::observe::observe" "Marks a TorqueState Faulted when a result is a status reply reporting a hardware fault; leaves it otherwise." "Rust fn, pub(crate)" {
            tags "Code"
        }
        bus_after_torque_write = component "bus::observe::after_torque_write" "Applies a Torque Enable write: wanted state on success, Faulted on a hardware fault, disable() on any other error." "Rust fn, pub(crate)" {
            tags "Code"
        }
        bus_check_clear = component "bus::observe::check_clear" "Reads Hardware Error Status where the model has it, else pings; clears to Off only on a clean answer, non-zero bits return a Status error." "Rust fn, pub(crate)" {
            tags "Code"
        }
    }

    group "bus: Wire" {
        bus_wire_struct = component "bus::wire::Wire" "Wire<'b, P, T, C>: ping, read/write (by Register or address), read_value / write_value (encode/decode up to 4 bytes), scan, sync_write, read_many; sync_read on P2 only; retries unicast on timeout or protocol error." "Rust struct, pub(crate)" {
            tags "Code"
        }
        bus_read_many = component "bus::wire::ReadMany" "How a multi-read reaches the wire: AnyProtocol (Easy tier, Sync Read on P2, one Read per Id on P1) or SyncRead (P2 explicit tier only)." "Rust trait + 2 strategy structs, pub(crate)" {
            tags "Code"
        }
    }

    // C3 relationships
    bus_core -> bus_setup "Builds its actuator list with" "declare / autoscan"
    bus_core -> bus_handle "Lends one at a time" "&mut borrow"
    bus_core -> bus_wire "Splits into a Wire per call"
    bus_dyn -> bus_setup "Builds its actuator list with"
    bus_dyn -> bus_handle "Lends one at a time" "&mut borrow"
    bus_dyn -> bus_multi "Reuses the generic helpers of"
    bus_dyn -> bus_wire "Splits into a Wire per call"
    bus_multi -> bus_core "Adds methods to" "impl blocks"
    bus_multi -> bus_group "Validates, stages and groups targets with"
    bus_multi -> bus_observe "Updates torque states through"
    bus_group -> bus_wire "Sends one instruction per (addr, size) group via"
    bus_group -> bus_observe "Marks faulted targets through"
    bus_handle -> bus_wire "Reads and writes registers via"
    bus_handle -> bus_observe "Checks every reply with"
    bus_setup -> bus_wire "Pings, scans and reads EEPROM via"
    bus_observe -> bus_wire "Reads Hardware Error Status or pings via"

    // C4 relationships
    bus_bus_struct -> bus_config "Holds"
    bus_bus_struct -> bus_default_clock "Defaults its clock to"
    bus_bus_struct -> bus_fixed_slots "Exposes its storage as"
    bus_bus_struct -> bus_actuator_ref "Returns from actuator(id)"
    bus_bus_struct -> bus_declare "Calls"
    bus_bus_struct -> bus_autoscan "Calls"
    bus_bus_struct -> bus_find "Looks up Ids with"
    bus_bus_struct -> bus_id_set "Returns from ids()"
    bus_serial_bus -> bus_bus_struct "Is an alias of"
    bus_fixed_slots -> bus_slots "Implements"
    bus_dyn_slots -> bus_slots "Implements"
    bus_dynbus_struct -> bus_dyn_slots "Exposes its storage as"
    bus_dynbus_struct -> bus_config "Holds"
    bus_dynbus_struct -> bus_multi_easy "Delegates multi-actuator calls to"
    bus_dynbus_struct -> bus_actuator_ref "Returns from actuator(id)"
    bus_actuator_ref -> bus_wire_struct "Owns for its lifetime"
    bus_actuator_ref -> bus_observe_fn "Checks reads and goal writes with"
    bus_actuator_ref -> bus_after_torque_write "Applies torque writes with"
    bus_actuator_ref -> bus_check_clear "Implements clear_fault with"
    bus_raw -> bus_actuator_ref "Extends"
    bus_raw -> bus_observe_fn "Checks raw replies with"
    bus_declare -> bus_init_one "Initializes each actuator with"
    bus_autoscan -> bus_init_one "Initializes each actuator with"
    bus_declare -> bus_slots "Clears and fills"
    bus_autoscan -> bus_slots "Clears and fills"
    bus_autoscan -> bus_wire_struct "Scans with"
    bus_declare -> bus_wire_struct "Pings with"
    bus_init_one -> bus_wire_struct "Reads firmware, limits and mode with"
    bus_multi_easy -> bus_field_markers "Selects registers with"
    bus_multi_easy -> bus_group_validate "Validates targets with"
    bus_multi_easy -> bus_group_groups "Builds Sync Write groups with"
    bus_multi_easy -> bus_read_each "Reads strictly with"
    bus_multi_easy -> bus_after_torque_write "Applies torque writes with"
    bus_multi_easy -> bus_first_error "Reports the error to return with"
    bus_sync_read -> bus_read_many "Uses the SyncRead strategy of"
    bus_partial -> bus_read_each "Reads with OnFailedGroup::Share via"
    bus_field_markers -> bus_field_traits "Implement"
    bus_group_validate -> bus_stage "Fills"
    bus_group_groups -> bus_stage "Groups by key"
    bus_read_each -> bus_read_many "Reaches the wire through"
    bus_read_each -> bus_observe_fn "Marks faulted targets with"
    bus_read_each -> bus_first_error "Feeds"
    bus_read_many -> bus_wire_struct "Calls read_many or sync_read on"
    bus_check_clear -> bus_wire_struct "Reads or pings via"
}

// Relationships to C2 elements of workspace.dsl
dxl.lib.bus_core -> dxl.lib.bus "Realizes"
dxl.lib.bus_dyn -> dxl.lib.bus "Realizes, without the N cap (std)"
dxl.lib.bus_handle -> dxl.lib.handle "Realizes"
dxl.lib.bus_multi -> dxl.lib.multi "Realizes"
dxl.lib.bus_group -> dxl.lib.multi "Realizes the (addr, size) grouping of"
dxl.lib.bus_setup -> dxl.lib.scan "Realizes"
dxl.lib.bus_observe -> dxl.lib.fault "Realizes"
app -> dxl.lib.bus_core "Creates, sets up and borrows handles from" "Rust API"
app -> dxl.lib.bus_multi "Reads and writes many actuators via" "Rust API"
dxl.lib.bus_core -> dxl.lib.kind "Owns actuators as" "heapless::Vec<ActuatorKind, N>"
dxl.lib.bus_core -> dxl.lib.transport "Is generic over" "T: Transport"
dxl.lib.bus_core -> port "Opens a serial port as SerialBus on" "std, serialport"
dxl.lib.bus_setup -> dxl.lib.kind "Instantiates by protocol, model number and firmware"
dxl.lib.bus_setup -> dxl.lib.errors "Fails with SetupError (Missing, WrongModel, UnsupportedModel, TooManyActuators, UnknownOperatingMode)"
dxl.lib.bus_handle -> dxl.lib.kind "Reads ModelData and the cache of"
dxl.lib.bus_handle -> dxl.lib.units "Takes and returns Position"
dxl.lib.bus_handle -> dxl.lib.errors "Returns ActuatorIsOff, ActuatorFaulted, NotPositionMode and limit errors as"
dxl.lib.bus_multi -> dxl.lib.units "Takes and returns Position"
dxl.lib.bus_group -> dxl.lib.models "Reads per-model register (addr, size) from"
dxl.lib.bus_group -> dxl.lib.errors "Returns NotOnBus, DuplicateId and BufferLength as"
dxl.lib.bus_observe -> dxl.lib.errors "Recognizes hardware faults in Error::Status"
dxl.lib.bus_wire -> dxl.lib.transport "Borrows per call" "&mut T"
dxl.lib.bus_wire -> actuators "Exchanges instruction and status packets with" "Dynamixel Protocol 1 / 2 via protocol::Link"

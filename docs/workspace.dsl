# C4 model of dxl_rs_v2, derived from ARCHITECTURE.md (not from the prototype in src/).
workspace "dxl_rs_v2" "no_std HAL for Dynamixel actuators (Protocol 1 & 2)." {

    !identifiers hierarchical

    configuration {
        scope softwaresystem
    }

    model {
        app = softwareSystem "Robot application" "Crate user (master app); owns the robot model, dxl_rs_v2 has none." "Rust" {
            tags "External"
        }
        maintainer = person "Crate maintainer" "Edits control tables and regenerates model code." {
            tags "External"
        }

        dxl = softwareSystem "dxl_rs_v2" "HAL for Dynamixel actuators, Protocol 1 & 2, blocking, heap-free." {

            lib = container "dxl_rs_v2 library" "Bus, actuator models, protocol and transport layers." "Rust crate, no_std + heap-free (std feature by default)" {
                tags "Library"

                bus = component "Bus<P, T, N>" "Owns actuators in heapless::Vec<ActuatorKind, N>; timeouts/retries config." "Rust struct, generic on Protocol + Transport" {
                    tags "Api"
                }
                handle = component "ActuatorRef" "Short-lived borrowed handle from bus.actuator(id); torque-off guard, raw register access." "Rust struct borrowing &mut Bus" {
                    tags "Api"
                }
                multi = component "Multi-actuator ops" "Easy (strategy picked by P) and explicit sync/bulk ops grouped by (addr, size); caller-owned buffers." "Rust impl blocks on Bus<P, T>" {
                    tags "Api"
                }
                scan = component "Autoscan / declared setup" "Pings the bus (P2 broadcast, P1 per ID), instantiates models, reads EEPROM limits." "Rust" {
                    tags "Api"
                }
                fault = component "Fault detection" "Checks every status packet error byte; marks actuator faulted, torque cache off." "Rust" {
                    tags "Api"
                }
                kind = component "ActuatorKind + Actuator trait" "Closed generated enum for heterogeneous storage; static dispatch via match." "Rust enum, generated" {
                    tags "Generated"
                }
                models = component "Model types" "Xm430W350, Mx64, ...: registers as associated consts, cache (Id, torque, limits)." "Rust, generated, families behind Cargo features" {
                    tags "Generated"
                }
                units = component "Typed values" "Id (0..=252), Position and SI newtypes with ranges; raw access kept." "Rust newtypes" {
                    tags "Core"
                }
                protocol = component "Protocol P1 | P2" "Packet encode/decode, checksum/CRC, status replies; RX cleared before TX, broadcast never waits." "Rust, compile-time generic" {
                    tags "Core"
                }
                transport = component "Transport" "Raw byte I/O over T: Read + Write; serialport std adapter by default." "embedded-io (blocking)" {
                    tags "Core"
                }
                errors = component "Errors" "Transport / protocol / status packet / validation, ActuatorIsOff, UnsupportedModel." "thiserror (no_std)" {
                    tags "Core"
                }
            }

            xtask = container "xtask codegen" "Generates committed .rs model files and ActuatorKind from control tables." "Rust binary, cargo xtask" {
                tags "Codegen"
            }

            tables = container "Control tables" "Per-model register tables plus model-number to table map." "RON files: control_tables/*.ron, dynamixel.ron" {
                tags "Data"
            }
        }

        port = softwareSystem "Serial adapter / UART" "OS serial port (serialport crate) or embedded UART in no_std." "USB-serial / UART" {
            tags "External"
        }
        actuators = softwareSystem "Dynamixel actuators" "Half-duplex TTL / RS-485 bus, Protocol 1 or 2 firmware." "Dynamixel hardware" {
            tags "External"
        }

        # Container-level relationships
        app -> dxl.lib "Scans, reads and writes actuators via" "Rust API"
        maintainer -> dxl.xtask "Runs" "cargo xtask"
        maintainer -> dxl.tables "Edits"
        dxl.xtask -> dxl.tables "Parses" "serde + ron"
        dxl.xtask -> dxl.lib "Writes generated model sources into" ".rs files, committed"
        dxl.lib -> port "Sends and receives bytes via" "embedded-io Read + Write"
        port -> actuators "Drives" "half-duplex TTL / RS-485"

        # Component-level relationships
        app -> dxl.lib.bus "Creates, configures and autoscans" "Rust API"
        app -> dxl.lib.handle "Controls single actuators via" "Rust API"
        app -> dxl.lib.multi "Reads/writes many actuators via" "Rust API"
        app -> dxl.lib.models "Calls model-specific methods on" "match on ActuatorKind"
        dxl.lib.bus -> dxl.lib.kind "Owns actuators as"
        dxl.lib.bus -> dxl.lib.handle "Lends"
        dxl.lib.bus -> dxl.lib.scan "Builds its actuator list with"
        dxl.lib.bus -> dxl.lib.protocol "Is generic over" "P: Protocol"
        dxl.lib.bus -> dxl.lib.transport "Is generic over" "T: Read + Write"
        dxl.lib.handle -> dxl.lib.kind "Dispatches to"
        dxl.lib.handle -> dxl.lib.protocol "Sends unicast instructions via"
        dxl.lib.handle -> dxl.lib.units "Takes and returns"
        dxl.lib.multi -> dxl.lib.models "Reads register addresses from"
        dxl.lib.multi -> dxl.lib.protocol "Encodes sync/bulk instructions via"
        dxl.lib.multi -> dxl.lib.units "Takes and returns"
        dxl.lib.scan -> dxl.lib.protocol "Pings via"
        dxl.lib.scan -> dxl.lib.kind "Instantiates by model number"
        dxl.lib.scan -> dxl.lib.errors "Fails with UnsupportedModel"
        dxl.lib.kind -> dxl.lib.models "Wraps one variant per"
        dxl.lib.protocol -> dxl.lib.transport "Writes and reads packets via"
        dxl.lib.protocol -> dxl.lib.fault "Hands status error byte to"
        dxl.lib.protocol -> dxl.lib.errors "Reports protocol / status errors as"
        dxl.lib.fault -> dxl.lib.kind "Marks actuator faulted in"
        dxl.lib.fault -> dxl.lib.errors "Raises hardware error as"
        dxl.lib.handle -> dxl.lib.errors "Returns ActuatorIsOff / validation as"
        dxl.lib.transport -> dxl.lib.errors "Reports I/O and timeouts as"
        dxl.lib.transport -> port "Reads and writes bytes on" "embedded-io / serialport"
        dxl.xtask -> dxl.lib.models "Generates"
        dxl.xtask -> dxl.lib.kind "Generates"
    }

    views {
        container dxl "Containers" "dxl_rs_v2 containers and their environment." {
            include * actuators
            autoLayout lr
        }

        component dxl.lib "LibraryComponents" "Layers of the dxl_rs_v2 library crate." {
            include *
            autoLayout tb
        }

        styles {
            element "Element" {
                color #ffffff
            }
            element "Person" {
                shape Person
                background #08427b
            }
            element "Software System" {
                background #1168bd
            }
            element "External" {
                background #8a8a8a
            }
            element "Container" {
                background #438dd5
            }
            element "Library" {
                background #2b6cb0
            }
            element "Codegen" {
                background #b7791f
                shape Hexagon
            }
            element "Data" {
                background #c05621
                shape Cylinder
            }
            element "Component" {
                background #85bbf0
                color #000000
            }
            element "Api" {
                background #63b3ed
            }
            element "Generated" {
                background #f6ad55
                shape Component
            }
            element "Core" {
                background #a3bffa
            }
        }
    }
}

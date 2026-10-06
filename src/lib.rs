#![allow(dead_code)]

pub mod port_handler;

pub mod actuator {
    use std::{io::Error, marker::PhantomData};

    use crate::port_handler::Baud;

    enum Model {
        Rx28,
        Mx64,
        Ax12,
    }

    impl Model {
        pub fn number(self) -> u16 {
            match self {
                Model::Rx28 => 28,
                Model::Mx64 => 64,
                Model::Ax12 => 12,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Bus {
        Ttl,
        Rs485,
    }

    enum Protocol {
        Protocol1,
        Protocol2,
    }

    #[derive(Default)]
    enum OperatingMode {
        Current,
        Velocity,
        #[default]
        Position,
        ExtendedPosition,
        CurrentBasedPosition,
        Pwm,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct DriveMode(u8);

    impl DriveMode {
        /// Bit 0: 0 = CCW positive (normal), 1 = CW positive (reverse).
        pub const REVERSE: Self = Self(1 << 0);
        /// Bit 1: 0 = master, 1 = slave (dual-joint follower).
        pub const SLAVE: Self = Self(1 << 1);
        /// Bit 2: 0 = velocity-based profile, 1 = time-based profile.
        pub const TIME_PROFILE: Self = Self(1 << 2);
        /// Bit 3: 0 = normal, 1 = torque on by goal update.
        pub const TORQUE_ON_BY_GOAL: Self = Self(1 << 3);

        pub const NORMAL: Self = Self(0);

        pub const fn from_bits(bits: u8) -> Self {
            Self(bits)
        }
        pub const fn bits(self) -> u8 {
            self.0
        }

        pub const fn contains(self, other: Self) -> bool {
            self.0 & other.0 == other.0
        }
        pub const fn union(self, other: Self) -> Self {
            Self(self.0 | other.0)
        }
        pub const fn difference(self, other: Self) -> Self {
            Self(self.0 & !other.0)
        }

        pub const fn is_reverse(self) -> bool {
            self.contains(Self::REVERSE)
        }
        pub const fn is_slave(self) -> bool {
            self.contains(Self::SLAVE)
        }
        pub const fn is_time_profile(self) -> bool {
            self.contains(Self::TIME_PROFILE)
        }
    }

    impl core::ops::BitOr for DriveMode {
        type Output = Self;
        fn bitor(self, rhs: Self) -> Self {
            self.union(rhs)
        }
    }

    impl core::ops::BitOrAssign for DriveMode {
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0;
        }
    }
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Shutdown(u8);

    impl Shutdown {
        /// Bit 0: input voltage outside the min/max voltage limits.
        pub const INPUT_VOLTAGE: Self = Self(1 << 0);
        /// Bit 2: internal temperature above the temperature limit.
        pub const OVERHEATING: Self = Self(1 << 2);
        /// Bit 3: motor encoder fault.
        pub const MOTOR_ENCODER: Self = Self(1 << 3);
        /// Bit 4: electrical shock on the circuit, or insufficient power to drive.
        pub const ELECTRICAL_SHOCK: Self = Self(1 << 4);
        /// Bit 5: sustained load exceeding the motor's rated output.
        pub const OVERLOAD: Self = Self(1 << 5);

        /// Factory default: 0x34 (overload | electrical shock | overheating).
        pub const DEFAULT: Self =
            Self(Self::OVERLOAD.0 | Self::ELECTRICAL_SHOCK.0 | Self::OVERHEATING.0);

        pub const NONE: Self = Self(0);

        pub const fn from_bits(bits: u8) -> Self {
            Self(bits)
        }
        pub const fn bits(self) -> u8 {
            self.0
        }

        pub const fn contains(self, other: Self) -> bool {
            self.0 & other.0 == other.0
        }
        pub const fn union(self, other: Self) -> Self {
            Self(self.0 | other.0)
        }
        pub const fn difference(self, other: Self) -> Self {
            Self(self.0 & !other.0)
        }
    }

    impl core::ops::BitOr for Shutdown {
        type Output = Self;
        fn bitor(self, rhs: Self) -> Self {
            self.union(rhs)
        }
    }

    impl core::ops::BitOrAssign for Shutdown {
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0;
        }
    }

    struct Dynamixel<S> {
        model: Model,
        id: u8,
        baud_rate: Baud, // todo: if baud not the same as in port, update dxl
        // with a prompt. if refused by user, abort and explain the issue
        protocol: Protocol,
        operating_mode: OperatingMode,
        _torque_state: PhantomData<S>,
    }

    struct TorqueOff;
    struct TorqueOn;
    impl<S> Dynamixel<S> {
        pub fn model_number(&self) -> Result<u16, Error> {
            todo!()
        }
        pub fn model_information(&self) -> Result<u32, Error> {
            todo!()
        }
        pub fn firmware_version(&self) -> Result<u8, Error> {
            todo!()
        }
        pub fn id(&self) -> Result<u8, Error> {
            todo!()
        }
        pub fn baud_rate(&self) -> Result<Baud, Error> {
            todo!()
        }
        pub fn return_delay_time(&self) -> Result<u8, Error> {
            todo!()
        }
        pub fn drive_mode(&self) -> Result<DriveMode, Error> {
            todo!()
        }
        pub fn operating_mode(&self) -> Result<OperatingMode, Error> {
            todo!()
        }
        pub fn secondary_id(&self) -> Result<u8, Error> {
            todo!()
        }
        pub fn protocol_type(&self) -> Result<Protocol, Error> {
            todo!()
        }
        pub fn homing_offset(&self) -> Result<i32, Error> {
            todo!()
        }
        pub fn moving_threshold(&self) -> Result<u32, Error> {
            todo!()
        }
        pub fn temperature_limit(&self) -> Result<u8, Error> {
            todo!()
        }
        pub fn max_voltage_limit(&self) -> Result<u16, Error> {
            todo!()
        }
        pub fn min_voltage_limit(&self) -> Result<u16, Error> {
            todo!()
        }
        pub fn pwm_limit(&self) -> Result<u16, Error> {
            todo!()
        }
        pub fn current_limit(&self) -> Result<u16, Error> {
            todo!()
        }
        pub fn velocity_limit(&self) -> Result<u32, Error> {
            todo!()
        }
        pub fn max_position_limit(&self) -> Result<u32, Error> {
            todo!()
        }
        pub fn min_position_limit(&self) -> Result<u32, Error> {
            todo!()
        }
        pub fn shutdown(&self) -> Result<Shutdown, Error> {
            todo!()
        }
    }

    // ---- Writes: EEPROM only writable with torque off ----
    impl Dynamixel<TorqueOff> {
        pub fn set_id(&mut self, _id: u8) -> Result<(), Error> {
            todo!()
        }
        pub fn set_baud_rate(&mut self, _b: Baud) -> Result<(), Error> {
            todo!()
        }
        pub fn set_return_delay_time(&mut self, _t: u8) -> Result<(), Error> {
            todo!()
        }
        pub fn set_drive_mode(&mut self, _m: DriveMode) -> Result<(), Error> {
            todo!()
        }
        pub fn set_operating_mode(&mut self, _m: OperatingMode) -> Result<(), Error> {
            todo!()
        }
        pub fn set_secondary_id(&mut self, _id: u8) -> Result<(), Error> {
            todo!()
        }
        pub fn set_protocol_type(&mut self, _p: Protocol) -> Result<(), Error> {
            todo!()
        }
        pub fn set_homing_offset(&mut self, _v: i32) -> Result<(), Error> {
            todo!()
        }
        pub fn set_moving_threshold(&mut self, _v: u32) -> Result<(), Error> {
            todo!()
        }
        pub fn set_temperature_limit(&mut self, _v: u8) -> Result<(), Error> {
            todo!()
        }
        pub fn set_max_voltage_limit(&mut self, _v: u16) -> Result<(), Error> {
            todo!()
        }
        pub fn set_min_voltage_limit(&mut self, _v: u16) -> Result<(), Error> {
            todo!()
        }
        pub fn set_pwm_limit(&mut self, _v: u16) -> Result<(), Error> {
            todo!()
        }
        pub fn set_current_limit(&mut self, _v: u16) -> Result<(), Error> {
            todo!()
        }
        pub fn set_velocity_limit(&mut self, _v: u32) -> Result<(), Error> {
            todo!()
        }
        pub fn set_max_position_limit(&mut self, _v: u32) -> Result<(), Error> {
            todo!()
        }
        pub fn set_min_position_limit(&mut self, _v: u32) -> Result<(), Error> {
            todo!()
        }
        pub fn set_shutdown(&mut self, _s: Shutdown) -> Result<(), Error> {
            todo!()
        }

        pub fn enable_torque(self) -> Result<Dynamixel<TorqueOn>, Error> {
            todo!()
        }
    }

    impl Dynamixel<TorqueOn> {
        fn goal_position(&mut self, p: u32) -> Result<(), Error> {
            todo!()
        }
        fn disable_torque(self) -> Result<Dynamixel<TorqueOff>, Error> {
            todo!()
        }
    }

    impl<S> Dynamixel<S> {
        fn present_position(&self) -> Result<u32, Error> {
            todo!()
        } // reads always fine
    }
}

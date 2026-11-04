//! [`HardwareErrors`] and [`Tick`].

/// Hardware Error Status: the faults an actuator reports, as named flags.
/// Read-only.
///
/// Bits from XM430-W350 Hardware Error Status(70)
/// (<https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>): bit 0 Input
/// Voltage, bit 2 Overheating, bit 3 Motor Encoder, bit 4 Electrical Shock,
/// bit 5 Overload; bits 1, 6 and 7 unused. Decoding never fails: an unused
/// bit stays in [`raw`](Self::raw).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HardwareErrors(u8);

impl HardwareErrors {
    pub(crate) const INPUT_VOLTAGE: u8 = 1 << 0;
    pub(crate) const OVERHEATING: u8 = 1 << 2;
    pub(crate) const ENCODER: u8 = 1 << 3;
    pub(crate) const ELECTRICAL_SHOCK: u8 = 1 << 4;
    pub(crate) const OVERLOAD: u8 = 1 << 5;

    /// The flags of raw register value `raw`.
    pub const fn from_raw(raw: u8) -> Self {
        Self(raw)
    }

    /// The raw register value, unused bits included.
    pub const fn raw(self) -> u8 {
        self.0
    }

    /// Input voltage outside the configured operating voltage.
    pub const fn input_voltage(self) -> bool {
        self.0 & Self::INPUT_VOLTAGE != 0
    }

    /// Internal temperature above the configured operating temperature.
    pub const fn overheating(self) -> bool {
        self.0 & Self::OVERHEATING != 0
    }

    /// Motor encoder malfunction.
    pub const fn encoder(self) -> bool {
        self.0 & Self::ENCODER != 0
    }

    /// Electrical shock on the circuit, or not enough power for the motor.
    pub const fn electrical_shock(self) -> bool {
        self.0 & Self::ELECTRICAL_SHOCK != 0
    }

    /// Persistent load above the maximum output.
    pub const fn overload(self) -> bool {
        self.0 & Self::OVERLOAD != 0
    }

    /// No fault flag set. Unused bits are ignored.
    pub const fn is_empty(self) -> bool {
        self.0
            & (Self::INPUT_VOLTAGE
                | Self::OVERHEATING
                | Self::ENCODER
                | Self::ELECTRICAL_SHOCK
                | Self::OVERLOAD)
            == 0
    }
}

impl HardwareErrors {
    /// The flags without `bit`.
    pub(crate) const fn without(self, bit: u8) -> Self {
        Self(self.0 & !bit)
    }

    /// Each fault bit with its name, lowest bit first.
    const NAMES: [(u8, &'static str); 5] = [
        (Self::INPUT_VOLTAGE, "an input voltage error"),
        (Self::OVERHEATING, "overheating"),
        (Self::ENCODER, "a motor encoder malfunction"),
        (Self::ELECTRICAL_SHOCK, "an electrical shock"),
        (Self::OVERLOAD, "an overload"),
    ];
}

/// The set faults by name, e.g. "an input voltage error, overheating";
/// "no fault" when none is set. Unused bits are left out.
impl core::fmt::Display for HardwareErrors {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut set = Self::NAMES
            .iter()
            .filter(|(bit, _)| self.0 & bit != 0)
            .map(|(_, name)| name);
        match set.next() {
            None => f.write_str("no fault"),
            Some(first) => {
                f.write_str(first)?;
                set.try_for_each(|name| write!(f, ", {name}"))
            }
        }
    }
}

/// Realtime Tick: the actuator's millisecond clock. Read-only.
///
/// It wraps: XM430-W350 Realtime Tick(120) counts 0 to 32,767 ms
/// (<https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>). The modulus
/// is passed to [`elapsed_since`](Self::elapsed_since) because it comes
/// from the model's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tick(u16);

impl Tick {
    /// The tick of raw register value `raw`.
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    /// The raw register value, in milliseconds.
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Milliseconds from `earlier` to `self` on a clock that wraps at
    /// `modulus` (32,768 for a clock counting 0 to 32,767). A `modulus` of 0
    /// means 65,536. Assumes less than one wrap between the two readings.
    pub const fn elapsed_since(self, earlier: Tick, modulus: u16) -> u16 {
        let modulus = if modulus == 0 {
            1 << 16
        } else {
            modulus as u32
        };
        let now = self.0 as u32 % modulus;
        let then = earlier.0 as u32 % modulus;
        ((now + modulus - then) % modulus) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_bit_sets_one_flag() {
        let flags = |e: HardwareErrors| {
            [
                e.input_voltage(),
                e.overheating(),
                e.encoder(),
                e.electrical_shock(),
                e.overload(),
            ]
        };
        assert_eq!(
            flags(HardwareErrors::from_raw(1 << 0)),
            [true, false, false, false, false]
        );
        assert_eq!(
            flags(HardwareErrors::from_raw(1 << 2)),
            [false, true, false, false, false]
        );
        assert_eq!(
            flags(HardwareErrors::from_raw(1 << 3)),
            [false, false, true, false, false]
        );
        assert_eq!(
            flags(HardwareErrors::from_raw(1 << 4)),
            [false, false, false, true, false]
        );
        assert_eq!(
            flags(HardwareErrors::from_raw(1 << 5)),
            [false, false, false, false, true]
        );
        for unused in [1, 6, 7] {
            let e = HardwareErrors::from_raw(1 << unused);
            assert_eq!(flags(e), [false; 5]);
            assert!(e.is_empty());
            assert_eq!(e.raw(), 1 << unused);
        }
        assert!(HardwareErrors::default().is_empty());
        let both = HardwareErrors::from_raw(0b0010_0100);
        assert!(both.overheating() && both.overload() && !both.is_empty());
    }

    #[test]
    fn elapsed_wraps() {
        assert_eq!(
            Tick::from_raw(5).elapsed_since(Tick::from_raw(32_760), 32_768),
            13
        );
        assert_eq!(
            Tick::from_raw(100).elapsed_since(Tick::from_raw(40), 32_768),
            60
        );
        assert_eq!(
            Tick::from_raw(7).elapsed_since(Tick::from_raw(7), 32_768),
            0
        );
        assert_eq!(
            Tick::from_raw(2).elapsed_since(Tick::from_raw(65_535), 0),
            3
        );
        assert_eq!(Tick::from_raw(9).raw(), 9);
    }
}

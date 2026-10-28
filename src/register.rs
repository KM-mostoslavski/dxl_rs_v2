//! Register locations inside a control table.

/// Whether a register can be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "RW and Ram are built by X1's generated reg() calls"
)]
pub enum Access {
    /// Read only.
    R,
    /// Read and write.
    RW,
}

/// Which memory area of the control table a register sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "RW and Ram are built by X1's generated reg() calls"
)]
pub enum Area {
    /// Kept across power cycles; writable only with torque off.
    Eeprom,
    /// Lost at power off; resets to its default at power on.
    Ram,
}

/// Where a register lives in an actuator's control table.
///
/// Generated model types expose one of these per register as an associated
/// `const`. Users never handle them: wrapper methods do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code, reason = "fields read by bus-1")]
pub struct Register {
    /// Control-table address.
    pub(crate) addr: u16,
    /// Width in bytes: 1, 2 or 4 for every register the core code encodes.
    pub(crate) size: u8,
    /// Whether the register can be written.
    pub(crate) access: Access,
    /// The memory area the register sits in.
    pub(crate) area: Area,
}

impl Register {
    /// Register at byte `addr`, `size` bytes wide, with placeholder access
    /// ([`Access::R`]) and area ([`Area::Eeprom`]).
    ///
    /// Kept only so today's generated code builds; generated code builds
    /// registers with its own `reg()` helper instead.
    #[doc(hidden)]
    #[allow(
        dead_code,
        reason = "unused once X1 regenerates with reg(); removed in core-2"
    )]
    pub(crate) const fn new(addr: u16, size: u8) -> Self {
        Self {
            addr,
            size,
            access: Access::R,
            area: Area::Eeprom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shim_keeps_address_and_size() {
        let reg = Register::new(116, 4);
        assert_eq!(reg.addr, 116);
        assert_eq!(reg.size, 4);
        assert_eq!(reg.access, Access::R);
        assert_eq!(reg.area, Area::Eeprom);
    }

    #[test]
    fn access_and_area_are_comparable() {
        assert_ne!(Access::R, Access::RW);
        assert_ne!(Area::Eeprom, Area::Ram);
    }
}

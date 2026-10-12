//! Register locations inside a control table.

/// Where a register lives in an actuator's control table.
///
/// Generated model types expose one of these per register as an associated
/// `const`. Users never handle them: wrapper methods do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "fields are read by the protocol module")
)]
pub(crate) struct Register {
    pub(crate) addr: u16,
    pub(crate) size: u8,
}

impl Register {
    /// Register at byte `addr`, `size` bytes wide.
    pub(crate) const fn new(addr: u16, size: u8) -> Self {
        Self { addr, size }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_address_and_size() {
        let reg = Register::new(116, 4);
        assert_eq!((reg.addr, reg.size), (116, 4));
    }
}

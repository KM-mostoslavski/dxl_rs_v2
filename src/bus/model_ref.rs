//! The Tier 2 handle: one actuator typed by its model (SRS FR-3.5, FR-9.1).

use core::marker::PhantomData;

use crate::Transport;
use crate::actuator::{
    Actuator, Cache, Describe, HasCcwComplianceMargin, HasCcwComplianceSlope,
    HasCwComplianceMargin, HasCwComplianceSlope, HasDGain, HasLedBlue, HasLedGreen, HasLedRed,
    HasPGain, HasTorqueCtrlModeEnable, Model, ModelData,
};
use crate::error::Error;
use crate::field::{
    CcwComplianceMargin, CcwComplianceSlope, CwComplianceMargin, CwComplianceSlope, DGain, LedBlue,
    LedGreen, LedRed, PGain, TorqueCtrlModeEnable,
};
use crate::protocol::{Clock, Protocol};
use crate::units::{ComplianceMargin, ComplianceSlope, Gain};

use super::ActuatorRef;

/// One actuator of model `M`, from [`ActuatorRef::model`]: the Tier 2
/// handle.
///
/// It has a method for a model-specific register only where `M`'s control
/// table has that register (its `Has*` impl), so a call on a model without
/// it does not compile. The AX-12A has compliance and no P gain:
#[cfg_attr(
    feature = "ax",
    doc = r#"
```compile_fail,E0599
use dxl_rs_v2::models::Ax12a;
use dxl_rs_v2::{ActuatorRef, Clock, P1, Transport};

fn tune<T: Transport, C: Clock>(ax: &mut ActuatorRef<'_, P1, T, C>) {
    if let Some(mut ax) = ax.model::<Ax12a>() {
        let _ = ax.set_p_gain(dxl_rs_v2::Gain::from_raw(32));
    }
}
```
"#
)]
///
/// It borrows the [`ActuatorRef`]; drop it to make Tier 1 calls again.
pub struct ModelRef<'h, 'b, M, P: Protocol, T: Transport, C: Clock> {
    handle: &'h mut ActuatorRef<'b, P, T, C>,
    model: PhantomData<fn() -> M>,
}

impl<'b, P: Protocol, T: Transport, C: Clock> ActuatorRef<'b, P, T, C> {
    /// The Tier 2 handle of this actuator as model `M`, or `None` when it is
    /// another model. No I/O.
    #[cfg_attr(
        all(feature = "mock", feature = "ax"),
        doc = r#"
```
use dxl_rs_v2::models::{Ax12a, Xm430W350};
use dxl_rs_v2::{Bus, ComplianceSlope, Config, Error, Id, MockTime, MockTransport, ModelNumber, P1};

# let time = MockTime::new();
# let mut line = MockTransport::new(&time);
# line.reply_to(1, &[0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC])
#     .reply_to(2, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x0C, 0x00, 0xEE])
#     .reply_to(3, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x00, 0x00, 0xFA])
#     .reply_to(4, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0xFF, 0x03, 0xF8])
#     .reply_to(5, &[0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC]);
let mut bus = Bus::<P1, _, _>::new(line, time.ticker(10), Config::new());
let id = Id::new(1)?;
bus.declare([(id, ModelNumber(12))])?;
let Some(mut handle) = bus.actuator(id) else { unreachable!("declared above") };
assert!(handle.model::<Xm430W350>().is_none());
let Some(mut ax) = handle.model::<Ax12a>() else { unreachable!("an AX-12A") };
// Allowed with torque off: compliance does not move the actuator.
ax.set_cw_compliance_slope(ComplianceSlope::new(6)?)?;
# let (line, _) = bus.into_parts();
# line.assert_done();
# Ok::<(), Error>(())
```
"#
    )]
    pub fn model<M: Model>(&mut self) -> Option<ModelRef<'_, 'b, M, P, T, C>> {
        (self.data() == M::DATA).then_some(ModelRef {
            handle: self,
            model: PhantomData,
        })
    }
}

impl<M, P: Protocol, T: Transport, C: Clock> Describe for ModelRef<'_, '_, M, P, T, C> {
    fn data(&self) -> &'static ModelData {
        self.handle.data()
    }

    fn cache(&self) -> &Cache {
        self.handle.cache()
    }

    fn cache_mut(&mut self) -> &mut Cache {
        self.handle.cache_mut()
    }
}

/// `id`, `model_number`, `name`, `position_scale` and
/// `position_limits[_raw]` of the held actuator, as read at Setup and kept
/// true by its limit writes (`None` after a failed one, until the next
/// Setup or an acknowledged write of both).
impl<M, P: Protocol, T: Transport, C: Clock> Actuator for ModelRef<'_, '_, M, P, T, C> {}

/// Declares a Tier 2 getter and setter pair for a role.
macro_rules! tier2 {
    ($(
        $has:ident, $field:ident, $value:ty, $get:ident, $set:ident, $what:literal;
    )*) => {$(
        impl<M: $has, P: Protocol, T: Transport, C: Clock> ModelRef<'_, '_, M, P, T, C> {
            #[doc = concat!("Reads ", $what, ".")]
            ///
            /// # Errors
            ///
            /// Any error of the read.
            pub fn $get(&mut self) -> Result<$value, Error> {
                self.handle.read($field)
            }

            #[doc = concat!("Writes ", $what, ": one acknowledged write, allowed with torque off.")]
            ///
            /// # Errors
            ///
            /// Refused with nothing sent: `OutOfLimits` outside the
            /// e-Manual range the table records. Otherwise any error of the
            /// write.
            pub fn $set(&mut self, value: $value) -> Result<(), Error> {
                self.handle.write($field, value)
            }
        }
    )*};
}

tier2! {
    HasPGain, PGain, Gain, p_gain, set_p_gain,
        "Position P Gain (P Gain on MX Protocol 1), in register units";
    HasDGain, DGain, Gain, d_gain, set_d_gain,
        "Position D Gain (D Gain on MX Protocol 1), in register units";
    HasCwComplianceMargin, CwComplianceMargin, ComplianceMargin,
        cw_compliance_margin, set_cw_compliance_margin, "CW Compliance Margin";
    HasCcwComplianceMargin, CcwComplianceMargin, ComplianceMargin,
        ccw_compliance_margin, set_ccw_compliance_margin, "CCW Compliance Margin";
    HasCwComplianceSlope, CwComplianceSlope, ComplianceSlope,
        cw_compliance_slope, set_cw_compliance_slope, "CW Compliance Slope";
    HasCcwComplianceSlope, CcwComplianceSlope, ComplianceSlope,
        ccw_compliance_slope, set_ccw_compliance_slope, "CCW Compliance Slope";
    HasLedRed, LedRed, u8, led_red, set_led_red, "LED Red, a brightness";
    HasLedGreen, LedGreen, u8, led_green, set_led_green, "LED Green, a brightness";
    HasLedBlue, LedBlue, u8, led_blue, set_led_blue, "LED Blue, a brightness";
}

impl<M: HasTorqueCtrlModeEnable, P: Protocol, T: Transport, C: Clock> ModelRef<'_, '_, M, P, T, C> {
    /// Reads Torque Ctrl Mode Enable (MX-64 on Protocol 1). Setup read it
    /// once to record the mode; it is not written here, since changing it
    /// changes the operating mode Setup recorded.
    ///
    /// # Errors
    ///
    /// Any error of the read.
    pub fn torque_ctrl_mode_enable(&mut self) -> Result<bool, Error> {
        self.handle.read(TorqueCtrlModeEnable)
    }
}

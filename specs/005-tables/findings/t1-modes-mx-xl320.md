# T1 `operating_modes`, MX(2.0) + XL-320 group (T036)

Rules of T036–T041: `bounded` = position modes in which the page says the limits apply; `unbounded` = position modes in which it says they are not used; `no_position` = modes that do not follow Goal Position. Each table copies its own page.

## Differences from the expected lists (flag to the architect)

- `mx_28`: page gives `(bounded: [3], unbounded: [4], no_position: [1, 16])`, expected `(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])` (research R-15 / data-model). Its page lists only the values above; the page wins.

No value was left unplaced: on every page each listed mode is a non-position mode (current, velocity, PWM, wheel) or a position mode whose limit text says whether the limits apply. No reserved value appears.

## Per table

### `mx_28`

Page https://emanual.robotis.com/docs/en/dxl/mx/mx-28-2/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 1 "Velocity Control Mode (0° ~ 360°)"; 3 "Position Control Mode"; 4 "Extended Position Control Mode (Multi-turn)"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by Max Position Limit(48) and Min Position Limit(52)."
- Min/Max Position Limit(48, 52): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [1, 16]),`

### `mx_64`

Page https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(48, 52): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `mx_106`

Page https://emanual.robotis.com/docs/en/dxl/mx/mx-106-2/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(48, 52): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xl320`

Page https://emanual.robotis.com/docs/en/dxl/x/xl320/, fetched 2026-10-28.

- Value table (Control Mode(11)): 1 "Wheel Mode"; 2 "Joint Mode".
- CW/CCW Angle Limit(6, 8): "The angle limit allows the motion to be restrained." "CW Angle Limit: the minimum value of Goal Position(30)" "CCW Angle Limit: the maximum value of Goal Position(30)". Goal Position(30): "If the Goal Position(30) is set over the CW/CCW Angle Limit(6, 8), the Status Packet transmits Angle Limit Error Bit (0x01)"; "NOTE : If it is set to Wheel Mode, Goal Position value is not used."
- Placement: 2 Joint Mode is the only position mode and Goal Position is bounded by the CW/CCW limits → `bounded`; 1 Wheel Mode does not use Goal Position → `no_position`.
- Written: `operating_modes: (bounded: [2], unbounded: [], no_position: [1]),`


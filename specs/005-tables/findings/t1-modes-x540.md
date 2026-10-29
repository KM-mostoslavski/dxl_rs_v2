# T1 `operating_modes`, X540 group (T038)

Rules as T036.

## Differences from the expected lists (flag to the architect)

- None: every table's page gives the expected list.

No value was left unplaced: on every page each listed mode is a non-position mode (current, velocity, PWM, wheel) or a position mode whose limit text says whether the limits apply. No reserved value appears.

## Per table

### `xd540_t150`

Page https://emanual.robotis.com/docs/en/dxl/x/xd540-t150/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xd540_t270`

Page https://emanual.robotis.com/docs/en/dxl/x/xd540-t270/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xh540_v150`

Page https://emanual.robotis.com/docs/en/dxl/x/xh540-v150/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xh540_v270`

Page https://emanual.robotis.com/docs/en/dxl/x/xh540-v270/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xh540_w150`

Page https://emanual.robotis.com/docs/en/dxl/x/xh540-w150/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xh540_w270`

Page https://emanual.robotis.com/docs/en/dxl/x/xh540-w270/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xm540_w150`

Page https://emanual.robotis.com/docs/en/dxl/x/xm540-w150/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xm540_w270`

Page https://emanual.robotis.com/docs/en/dxl/x/xm540-w270/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xw540_h260`

Page https://emanual.robotis.com/docs/en/dxl/x/xw540-h260/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xw540_t140`

Page https://emanual.robotis.com/docs/en/dxl/x/xw540-t140/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xw540_t260`

Page https://emanual.robotis.com/docs/en/dxl/x/xw540-t260/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`


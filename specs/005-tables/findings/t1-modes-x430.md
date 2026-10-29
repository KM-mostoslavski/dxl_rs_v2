# T1 `operating_modes`, X430 group (T037)

Rules as T036. `xw430_t200` is not mapped; the rule applies to it too.

## Differences from the expected lists (flag to the architect)

- `xc430_w150`: page gives `(bounded: [3], unbounded: [4], no_position: [1, 16])`, expected `(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])` (research R-15 / data-model). Its page lists only the values above; the page wins.
- `xc430_w240`: page gives `(bounded: [3], unbounded: [4], no_position: [1, 16])`, expected `(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])` (research R-15 / data-model). Its page lists only the values above; the page wins.

No value was left unplaced: on every page each listed mode is a non-position mode (current, velocity, PWM, wheel) or a position mode whose limit text says whether the limits apply. No reserved value appears.

## Per table

### `2xc430_w250`

Page https://emanual.robotis.com/docs/en/dxl/x/2xc430-w250/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 1 "Velocity Control Mode (0° ~ 360°)"; 3 "Position Control Mode"; 4 "Extended Position Control Mode (Multi-turn)"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by Max Position Limit(48) and Min Position Limit(52)."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [1, 16]),`

### `2xl430_w250`

Page https://emanual.robotis.com/docs/en/dxl/x/2xl430-w250/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [1, 16]),`

### `xc430_w150`

Page https://emanual.robotis.com/docs/en/dxl/x/xc430-w150/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 1 "Velocity Control Mode (0° ~ 360°)"; 3 "Position Control Mode"; 4 "Extended Position Control Mode (Multi-turn)"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by Max Position Limit(48) and Min Position Limit(52)."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [1, 16]),`

### `xc430_w240`

Page https://emanual.robotis.com/docs/en/dxl/x/xc430-w240/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 1 "Velocity Control Mode (0° ~ 360°)"; 3 "Position Control Mode"; 4 "Extended Position Control Mode (Multi-turn)"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by Max Position Limit(48) and Min Position Limit(52)."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [1, 16]),`

### `xd430_t210`

Page https://emanual.robotis.com/docs/en/dxl/x/xd430-t210/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xd430_t350`

Page https://emanual.robotis.com/docs/en/dxl/x/xd430-t350/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xh430_v210`

Page https://emanual.robotis.com/docs/en/dxl/x/xh430-v210/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xh430_v350`

Page https://emanual.robotis.com/docs/en/dxl/x/xh430-v350/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xh430_w210`

Page https://emanual.robotis.com/docs/en/dxl/x/xh430-w210/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xh430_w350`

Page https://emanual.robotis.com/docs/en/dxl/x/xh430-w350/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xl430_w250`

Page https://emanual.robotis.com/docs/en/dxl/x/xl430-w250/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(48, 52) is used to configure the maximum movement range in Position Control Mode within a single rotation(0 ~ 4,095)." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode within a single turn." "The angle limits set through this control table item do not apply in Extended Position Control Mode."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [1, 16]),`

### `xm430_w210`

Page https://emanual.robotis.com/docs/en/dxl/x/xm430-w210/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xm430_w350`

Page https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xw430_t200`

Page https://emanual.robotis.com/docs/en/dxl/x/xw430-t200/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xw430_t333`

Page https://emanual.robotis.com/docs/en/dxl/x/xw430-t333/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`


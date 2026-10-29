# T1 `operating_modes`, X330 group (T039)

Rules as T036. Six `_fw52` tables use their model's page (see the note on each).

## Differences from the expected lists (flag to the architect)

- None: every table's page gives the expected list.

No value was left unplaced: on every page each listed mode is a non-position mode (current, velocity, PWM, wheel) or a position mode whose limit text says whether the limits apply. No reserved value appears.

## Per table

### `xc330_m181`

Page https://emanual.robotis.com/docs/en/dxl/x/xc330-m181/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xc330_m288`

Page https://emanual.robotis.com/docs/en/dxl/x/xc330-m288/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xc330_t181`

Page https://emanual.robotis.com/docs/en/dxl/x/xc330-t181/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xc330_t288`

Page https://emanual.robotis.com/docs/en/dxl/x/xc330-t288/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xl330_m077`

Page https://emanual.robotis.com/docs/en/dxl/x/xl330-m077/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xl330_m288`

Page https://emanual.robotis.com/docs/en/dxl/x/xl330-m288/, fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`

### `xc330_m181_fw52`

Page https://emanual.robotis.com/docs/en/dxl/x/xc330-m181/ (the model page of `xc330_m181`), fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`
- Firmware: the page gives one Operating Mode description with no firmware condition (no list for newer firmware only), so per T039 it applies to the `_fw52` table too.

### `xc330_m288_fw52`

Page https://emanual.robotis.com/docs/en/dxl/x/xc330-m288/ (the model page of `xc330_m288`), fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`
- Firmware: the page gives one Operating Mode description with no firmware condition (no list for newer firmware only), so per T039 it applies to the `_fw52` table too.

### `xc330_t181_fw52`

Page https://emanual.robotis.com/docs/en/dxl/x/xc330-t181/ (the model page of `xc330_t181`), fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`
- Firmware: the page gives one Operating Mode description with no firmware condition (no list for newer firmware only), so per T039 it applies to the `_fw52` table too.

### `xc330_t288_fw52`

Page https://emanual.robotis.com/docs/en/dxl/x/xc330-t288/ (the model page of `xc330_t288`), fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`
- Firmware: the page gives one Operating Mode description with no firmware condition (no list for newer firmware only), so per T039 it applies to the `_fw52` table too.

### `xl330_m077_fw52`

Page https://emanual.robotis.com/docs/en/dxl/x/xl330-m077/ (the model page of `xl330_m077`), fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`
- Firmware: the page gives one Operating Mode description with no firmware condition (no list for newer firmware only), so per T039 it applies to the `_fw52` table too.

### `xl330_m288_fw52`

Page https://emanual.robotis.com/docs/en/dxl/x/xl330-m288/ (the model page of `xl330_m288`), fetched 2026-10-28.

- Value table (Operating Mode(11)): 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode(Multi-turn)"; 5 "Current-based Position Control Mode"; 16 "PWM Control Mode (Voltage Control Mode)".
  - 3 description: "Operating position range is limited by the Max Position Limit(48) and the Min Position Limit(52)."
  - 4 description: "Note that Max Position Limit(48), Min Position Limit(52) are not used on Extended Position Control Mode."
- Min/Max Position Limit(52, 48): "The Min and Max Position Limit(52, 48) limit the maximum and minimum positions for Position Control Mode(Joint Mode) within the range of 1 rotation(0 ~ 4,095)." "Goal Position(116) will also be limited by be the position limit range." "These values are not used in Extended Position Control Mode and Current-based Position Control Mode." "NOTE : Max Position Limit(48) and Min Position Limit(52) are only used in Position Control Mode with a single turn."
- Written: `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),`
- Firmware: the page gives one Operating Mode description with no firmware condition (no list for newer firmware only), so per T039 it applies to the `_fw52` table too.


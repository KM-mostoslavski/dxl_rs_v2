# T1 `operating_modes`, PRO / PRO+ / RH group (T040)

Rules as T036. Pages fetched 2026-10-29.

- PRO (A) and PRO+: 0 Current, 1 Velocity, 16 PWM → `no_position`; 3 Position ("The position range is configured by the Max Position Limit(48) and the Min Position Limit(52)") → `bounded`; 4 Extended Position ("not limited by the Position Limit control table items"; limits "NOT used in Extended Position Control Mode") → `unbounded`.
- RH-P12-RN(A), its own (A) page: 0 Current → `no_position`; "1 ~ 4 Reserved" → no list; 5 Current based Position ("These values limit maximum and minimum positions in Current based Position Control Mode") → `bounded`.
- No difference from the expected lists; no value left unplaced.

## Per table

### `h42_20_s300_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/h42-20-s300-ra/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "These values limit maximum and minimum positions for Position Control Mode within the range of 1 rotation(-303,750 ~ 303,750)." "Unit | Value Range | 1 [pulse] | -303,750 ~ 303,750 | NOTE : Max Position Limit(48) and Min Position Limit(52) are NOT used in Extended Position Control Mode."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `h54_100_s500_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/h54-100-s500-ra/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "These values limit maximum and minimum positions for Position Control Mode within the range of 1 rotation(-501,923 ~ 501,923)." "Unit | Value Range | 1 [pulse] | -501,923 ~ 501,923 | NOTE : Max Position Limit(48) and Min Position Limit(52) are NOT used in Extended Position Control Mode."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `h54_200_s500_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/h54-200-s500-ra/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "These values limit maximum and minimum positions for Position Control Mode within the range of 1 rotation(-501,923 ~ 501,923)." "Unit | Value Range | 1 [pulse] | -501,923 ~ 501,923 | NOTE : Max Position Limit(48) and Min Position Limit(52) are NOT used in Extended Position Control Mode."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `m42_10_s260_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/m42-10-s260-ra/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "These values limit maximum and minimum positions for Position Control Mode within the range of 1 rotation(-263,187 ~ 263,187)." "Unit | Value Range | 1 [pulse] | -263,187 ~ 263,187 | NOTE : Max Position Limit(48) and Min Position Limit(52) are NOT used in Extended Position Control Mode."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `m54_40_s250_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/m54-40-s250-ra/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "These values limit maximum and minimum positions for Position Control Mode within the range of 1 rotation(-251,417 ~ 251,417)." "Unit | Value Range | 1 [pulse] | -251,417 ~ 251,417 | NOTE : Max Position Limit(48) and Min Position Limit(52) are NOT used in Extended Position Control Mode."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `m54_60_s250_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/m54-60-s250-ra/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "These values limit maximum and minimum positions for Position Control Mode within the range of 1 rotation(-251,417 ~ 251,417)." "Unit | Value Range | 1 [pulse] | -251,417 ~ 251,417 | NOTE : Max Position Limit(48) and Min Position Limit(52) are NOT used in Extended Position Control Mode."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `ph42_020_s300`

Page https://emanual.robotis.com/docs/en/dxl/p/ph42-020-s300-r/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "Unit | Range | 1 [pulse] | -303,750 ~ 303,750 | NOTE : The Max Position Limit(48) and Min Position Limit(52) are only used when the Operating Mode is the Position Control Mode (Joint Mode) which allows DYNAMIXEL for 1 rev only."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `ph54_100_s500`

Page https://emanual.robotis.com/docs/en/dxl/p/ph54-100-s500-r/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "Unit | Range | 1 [pulse] | -501,923 ~ 501,923 | NOTE : The Max Position Limit(48) and Min Position Limit(52) are only used when the Operating Mode is the Position Control Mode which allows DYNAMIXEL for 1 rev only."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `ph54_200_s500`

Page https://emanual.robotis.com/docs/en/dxl/p/ph54-200-s500-r/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "Unit | Range | 1 [pulse] | -501,923 ~ 501,923 | NOTE : The Max Position Limit(48) and Min Position Limit(52) are only used when the Operating Mode is the Position Control Mode (Joint Mode) which allows DYNAMIXEL for 1 rev only."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `pm42_010_s260`

Page https://emanual.robotis.com/docs/en/dxl/p/pm42-010-s260-r/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "Unit | Range | 1 [pulse] | -263,187 ~ 263,187 | NOTE : The Max Position Limit(48) and Min Position Limit(52) are only used when the Operating Mode is the Position Control Mode (Joint Mode) which allows DYNAMIXEL for 1 rev only."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `pm54_040_s250`

Page https://emanual.robotis.com/docs/en/dxl/p/pm54-040-s250-r/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "Unit | Range | 1 [pulse] | -251,417 ~ 251,417 | NOTE : The Max Position Limit(48) and Min Position Limit(52) are only used when the Operating Mode is the Position Control Mode (Joint Mode) which allows DYNAMIXEL for 1 rev only."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `pm54_060_s250`

Page https://emanual.robotis.com/docs/en/dxl/p/pm54-060-s250-r/.

- Mode value table: 0 "Current Control Mode"; 1 "Velocity Control Mode"; 3 "Position Control Mode"; 4 "Extended Position Control Mode"; 16 "PWM(Voltage) Control Mode".
- Limit text: "Unit | Range | 1 [pulse] | -251,417 ~ 251,417 | NOTE : The Max Position Limit(48) and Min Position Limit(52) are only used when the Operating Mode is the Position Control Mode (Joint Mode) which allows DYNAMIXEL for 1 rev only."
- Written: `operating_modes: (bounded: [3], unbounded: [4], no_position: [0, 1, 16]),`

### `rh_p12_rn`

Page https://emanual.robotis.com/docs/en/platform/rh_p12_rna/.

- Mode value table: 0 "Current Control Mode"; 1 ~ 4 "Reserved"; 5 "Current based Position Control Mode".
- Limit text: "These values limit maximum and minimum positions in Current based Position Control Mode within the range of 0 ~ 1,150."
- Written: `operating_modes: (bounded: [5], unbounded: [], no_position: [0]),`


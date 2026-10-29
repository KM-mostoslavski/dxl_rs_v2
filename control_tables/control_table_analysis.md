# dxl_rs Control-Table Analysis

Generated from `control_tables/*.ron` — **76 models** (excludes `dynamixel.ron` model-number map).

- Protocol 1 only (`protocols: [P1]`): **9**
- Protocol 2 only (`protocols: [P2]`): **38**
- Both protocols (`protocols: [P1, P2]`): **29**
- No `protocols` yet: **0**
  - P1 only: ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28
  - P2 only: h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xl320, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099
  - P1 and P2: 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260
- Distinct register names across all models: **142**
- Distinct control-table layouts (exact signatures): **18**

## 1. Exact-duplicate groups

Models with **byte-identical control tables** (same register set, same address, same size for every register). One trait/struct can serve each whole group with zero per-model divergence.

| # | Proto | Members | Regs | Model #s |
|---|-------|---------|------|----------|
| 1 | P2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 | 94 | 4040, 4050, 4010, 4000, 4020, 4030, 4160, 4170, 4130, 4120, 4140, 4150 |
| 2 | P1, P2 | xd430_t210, xd430_t350, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xm430_w210, xm430_w350 | 53 | 1011, 1001, 1050, 1040, 1010, 1000, 1030, 1020 |
| 3 | P1, P2 | xd540_t150, xd540_t270, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xm540_w150, xm540_w270 | 59 | 1111, 1101, 1150, 1140, 1110, 1100, 1130, 1120 |
| 4 | P1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 | 32 | 12, 300, 18, 28 |
| 5 | P2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra | 63 | 51201, 53769, 54025, 43289, 46097, 46353 |
| 6 | P2 | xc330_m181, xc330_m288, xc330_t181, xc330_t288, xl330_m077, xl330_m288 | 54 | 1230, 1240, 1210, 1220, 1190, 1200 |
| 7 | P2 | xc330_m181_fw52, xc330_m288_fw52, xc330_t181_fw52, xc330_t288_fw52, xl330_m077_fw52, xl330_m288_fw52 | 54 | 1230, 1240, 1210, 1220, 1190, 1200 |
| 8 | P1, P2 | 2xc430_w250, 2xl430_w250, xc430_w150, xc430_w240, xl430_w250 | 51 | 1160, 1090, 1070, 1080, 1060 |
| 9 | P1, P2 | xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 | 52 | 1270, 1310, 1180, 1170 |
| 10 | P2 | ph42_020_s300, ph54_100_s500, ph54_200_s500 | 66 | 2000, 2010, 2020 |
| 11 | P2 | pm42_010_s260, pm54_040_s250, pm54_060_s250 | 66 | 2100, 2110, 2120 |
| 12 | P1, P2 | mx_106, mx_64 | 54 | 321, 311 |
| 13 | P1 | mx_106_p1 | 39 | 320 |
| 14 | P1, P2 | mx_28 | 52 | 30 |
| 15 | P1 | mx_28_p1 | 35 | 29 |
| 16 | P1 | mx_64_p1 | 38 | 310 |
| 17 | P2 | rh_p12_rn | 63 | 35074 |
| 18 | P2 | xl320 | 31 | 350 |

## 2. Architectural tiers — the 4 RAM-layout generations

The 14 exact layouts collapse into **4 tiers** keyed by where `Goal Position` lives (the anchor that encodes the RAM-layout generation). Within a tier, tables share the same address space and differ only by which registers are present. Across tiers, even identically-named registers sit at different addresses — so a name-keyed trait works, but addresses must be per-model constants.

### T1 · Legacy layout (AX / RX-28 / XL320)
- 10 models, 5 exact layouts, Goal Position @ addr 30 size 2
- Register-count range: 31–39
- Common to all in tier: 25 regs · union 46
- Varies within tier (21): Alarm LED, CCW Compliance Margin, CCW Compliance Slope, CW Compliance Margin, CW Compliance Slope, Control Mode, Current, D Gain, Drive Mode, Goal Acceleration, Goal Torque, Hardware Error Status, I Gain, Lock, Multi Turn Offset, P Gain, Realtime Tick, Registered, Registered Instruction, Resolution Divider, Torque Ctrl Mode Enable
- Layouts: [ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28] | [mx_106_p1] | [mx_28_p1] | [mx_64_p1] | [xl320]

### T2 · Modern X-series (XM/XH/XD/XC/XL430/XL330/XW + MX2.0)
- 41 models, 8 exact layouts, Goal Position @ addr 116 size 4
- Register-count range: 51–59
- Common to all in tier: 48 regs · union 63
- Varies within tier (15): Acceleration Limit, BUS Watchdog, Bus Watchdog, Current Limit, External Port Data 1, External Port Data 2, External Port Data 3, External Port Mode 1, External Port Mode 2, External Port Mode 3, Goal Current, LED, PWM Slope, Present Current, Present Load
- Layouts: [xd430_t210, xd430_t350, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xm430_w210, xm430_w350] | [xd540_t150, xd540_t270, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xm540_w150, xm540_w270] | [xc330_m181, xc330_m288, xc330_t181, xc330_t288, xl330_m077, xl330_m288] | [xc330_m181_fw52, xc330_m288_fw52, xc330_t181_fw52, xc330_t288_fw52, xl330_m077_fw52, xl330_m288_fw52] | [2xc430_w250, 2xl430_w250, xc430_w150, xc430_w240, xl430_w250] | [xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260] | [mx_106, mx_64] | [mx_28]

### T3 · YM series
- 12 models, 1 exact layouts, Goal Position @ addr 532 size 4
- Register-count range: 94–94
- Common to all in tier: 94 regs · union 94
- Layouts: [ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099]

### T4 · Pro / P-series (H/M/PH/PM/RH)
- 13 models, 4 exact layouts, Goal Position @ addr 564 size 4
- Register-count range: 63–66
- Common to all in tier: 61 regs · union 67
- Varies within tier (6): Backup Ready, Drive Mode, Protocol Type, Secondary ID, Secondary(Shadow) ID, Startup Configuration
- Layouts: [h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra] | [ph42_020_s300, ph54_100_s500, ph54_200_s500] | [pm42_010_s260, pm54_040_s250, pm54_060_s250] | [rh_p12_rn]

### 2b. Cross-layout similarity matrix

Upper cell = **register-name overlap** (Jaccard %). Lower cell = **address-identical overlap** — of the registers two layouts share by name, the % that also match on `(addr,size)`. High name-overlap with LOW address-overlap (e.g. X-series vs Pro) means: same concepts, different memory map → a trait can unify the API but not the constants.

Legend: **L1**=ax_12_plus(6m,P1) · **L2**=mx_106_p1(1m,P1) · **L3**=mx_28_p1(1m,P1) · **L4**=mx_64_p1(1m,P1) · **L5**=xl320(1m,P2) · **L6**=xd430_t210(8m,P1, P2) · **L7**=xd540_t150(8m,P1, P2) · **L8**=xc330_m181(6m,P2) · **L9**=xc330_m181_fw52(6m,P2) · **L10**=2xc430_w250(5m,P1, P2) · **L11**=xw430_t200(5m,P1, P2) · **L12**=mx_106(2m,P1, P2) · **L13**=mx_28(1m,P1, P2) · **L14**=ym070_210_a051(12m,P2) · **L15**=h42_20_s300_ra(6m,P2) · **L16**=ph42_020_s300(3m,P2) · **L17**=pm42_010_s260(3m,P2) · **L18**=rh_p12_rn(1m,P2)

| name\addr | L1 | L2 | L3 | L4 | L5 | L6 | L7 | L8 | L9 | L10 | L11 | L12 | L13 | L14 | L15 | L16 | L17 | L18 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| L1 | **L1** | 65 | 72 | 67 | 66 | 23 | 21 | 23 | 23 | 26 | 22 | 23 | 25 | 10 | 19 | 18 | 18 | 19 |
| L2 | _100_ | **L2** | 90 | 97 | 67 | 24 | 22 | 24 | 24 | 27 | 23 | 24 | 26 | 11 | 20 | 19 | 19 | 19 |
| L3 | _100_ | _100_ | **L3** | 92 | 74 | 24 | 22 | 24 | 24 | 26 | 23 | 24 | 26 | 10 | 20 | 19 | 19 | 20 |
| L4 | _100_ | _100_ | _100_ | **L4** | 68 | 23 | 21 | 23 | 23 | 25 | 22 | 23 | 25 | 10 | 19 | 18 | 18 | 19 |
| L5 | _48_ | _43_ | _43_ | _43_ | **L5** | 27 | 25 | 27 | 27 | 30 | 26 | 27 | 30 | 11 | 22 | 21 | 21 | 22 |
| L6 | _6_ | _11_ | _6_ | _6_ | _6_ | **L6** | 90 | 98 | 98 | 93 | 98 | 95 | 88 | 44 | 76 | 78 | 75 | 76 |
| L7 | _6_ | _11_ | _6_ | _6_ | _6_ | _100_ | **L7** | 88 | 88 | 83 | 88 | 85 | 79 | 42 | 85 | 87 | 84 | 85 |
| L8 | _6_ | _11_ | _6_ | _6_ | _6_ | _100_ | _100_ | **L8** | 100 | 91 | 96 | 93 | 86 | 44 | 75 | 76 | 74 | 75 |
| L9 | _6_ | _11_ | _6_ | _6_ | _6_ | _98_ | _98_ | _98_ | **L9** | 91 | 96 | 93 | 86 | 44 | 75 | 76 | 74 | 75 |
| L10 | _6_ | _11_ | _6_ | _6_ | _5_ | _100_ | _100_ | _100_ | _98_ | **L10** | 91 | 88 | 94 | 41 | 70 | 72 | 70 | 70 |
| L11 | _7_ | _12_ | _6_ | _6_ | _6_ | _100_ | _100_ | _100_ | _98_ | _100_ | **L11** | 93 | 86 | 43 | 77 | 79 | 76 | 77 |
| L12 | _6_ | _11_ | _6_ | _6_ | _6_ | _100_ | _100_ | _100_ | _98_ | _100_ | _100_ | **L12** | 93 | 44 | 75 | 76 | 74 | 75 |
| L13 | _6_ | _11_ | _6_ | _6_ | _5_ | _100_ | _100_ | _100_ | _98_ | _100_ | _100_ | _100_ | **L13** | 40 | 69 | 71 | 69 | 69 |
| L14 | _9_ | _8_ | _8_ | _8_ | _8_ | _9_ | _9_ | _9_ | _9_ | _10_ | _9_ | _9_ | _10_ | **L14** | 38 | 42 | 40 | 38 |
| L15 | _7_ | _12_ | _6_ | _6_ | _6_ | _40_ | _41_ | _40_ | _40_ | _40_ | _40_ | _42_ | _43_ | _14_ | **L15** | 93 | 95 | 97 |
| L16 | _7_ | _12_ | _6_ | _6_ | _6_ | _42_ | _43_ | _42_ | _42_ | _43_ | _42_ | _44_ | _45_ | _13_ | _100_ | **L16** | 97 | 93 |
| L17 | _7_ | _12_ | _6_ | _6_ | _6_ | _41_ | _42_ | _41_ | _41_ | _42_ | _41_ | _43_ | _44_ | _13_ | _100_ | _100_ | **L17** | 95 |
| L18 | _7_ | _6_ | _6_ | _6_ | _6_ | _40_ | _41_ | _40_ | _40_ | _40_ | _40_ | _42_ | _43_ | _14_ | _100_ | _100_ | _100_ | **L18** |

## 3. Register universe — presence / address / size consistency

For every distinct register name: how many models have it, whether the **address** and **size** are the same in every model that has it, and the observed values. This is the core table for deciding what belongs in a shared trait vs a per-model override.

Legend: ✅ uniform · ⚠️ VARIES (address or size differs across models).

| Register | Models | Addr | Size | Addr values | Size values |
|----------|:------:|:----:|:----:|-------------|-------------|
| Firmware Version | 76/76 | ⚠️ | ✅ | 2, 6 | 1 |
| Goal Position | 76/76 | ⚠️ | ⚠️ | 30, 116, 532, 564 | 2, 4 |
| ID | 76/76 | ⚠️ | ✅ | 3, 7 | 1 |
| Max Voltage Limit | 76/76 | ⚠️ | ⚠️ | 13, 14, 32, 60 | 1, 2 |
| Min Voltage Limit | 76/76 | ⚠️ | ⚠️ | 12, 13, 34, 62 | 1, 2 |
| Model Number | 76/76 | ✅ | ✅ | 0 | 2 |
| Present Position | 76/76 | ⚠️ | ⚠️ | 36, 37, 132, 552, 580 | 2, 4 |
| Return Delay Time | 76/76 | ⚠️ | ✅ | 5, 9, 13 | 1 |
| Status Return Level | 76/76 | ⚠️ | ✅ | 15, 16, 17, 68, 516 | 1 |
| Torque Enable | 76/76 | ⚠️ | ✅ | 24, 64, 512 | 1 |
| Realtime Tick | 69/76 | ⚠️ | ✅ | 50, 120, 542, 568 | 2 |
| Registered Instruction | 67/76 | ⚠️ | ✅ | 16, 47, 69, 517 | 1 |
| Drive Mode | 66/76 | ⚠️ | ✅ | 10, 32 | 1 |
| Goal PWM | 66/76 | ⚠️ | ✅ | 100, 524, 548 | 2 |
| Goal Velocity | 66/76 | ⚠️ | ✅ | 104, 528, 552 | 4 |
| Homing Offset | 66/76 | ⚠️ | ✅ | 20, 52 | 4 |
| Indirect Address 1 | 66/76 | ⚠️ | ✅ | 168, 256 | 2 |
| Indirect Data 1 | 66/76 | ⚠️ | ✅ | 208, 224, 634 | 1 |
| Max Position Limit | 66/76 | ⚠️ | ✅ | 48, 76 | 4 |
| Min Position Limit | 66/76 | ⚠️ | ✅ | 52, 84 | 4 |
| Model Information | 66/76 | ✅ | ✅ | 2 | 4 |
| Moving Status | 66/76 | ⚠️ | ✅ | 123, 541, 571 | 1 |
| Moving Threshold | 66/76 | ⚠️ | ✅ | 24, 48 | 4 |
| Operating Mode | 66/76 | ⚠️ | ✅ | 11, 33 | 1 |
| PWM Limit | 66/76 | ⚠️ | ✅ | 36, 64 | 2 |
| Position D Gain | 66/76 | ⚠️ | ⚠️ | 80, 224, 528 | 2, 4 |
| Position I Gain | 66/76 | ⚠️ | ⚠️ | 82, 228, 530 | 2, 4 |
| Position P Gain | 66/76 | ⚠️ | ⚠️ | 84, 232, 532 | 2, 4 |
| Position Trajectory | 66/76 | ⚠️ | ✅ | 140, 560, 588 | 4 |
| Present Input Voltage | 66/76 | ⚠️ | ✅ | 144, 568, 592 | 2 |
| Present PWM | 66/76 | ⚠️ | ✅ | 124, 544, 572 | 2 |
| Present Velocity | 66/76 | ⚠️ | ✅ | 128, 548, 576 | 4 |
| Profile Acceleration | 66/76 | ⚠️ | ✅ | 108, 240, 556 | 4 |
| Profile Velocity | 66/76 | ⚠️ | ✅ | 112, 244, 560 | 4 |
| Velocity I Gain | 66/76 | ⚠️ | ⚠️ | 76, 212, 524 | 2, 4 |
| Velocity Limit | 66/76 | ⚠️ | ✅ | 44, 72 | 4 |
| Velocity P Gain | 66/76 | ⚠️ | ⚠️ | 78, 216, 526 | 2, 4 |
| Velocity Trajectory | 66/76 | ⚠️ | ✅ | 136, 564, 584 | 4 |
| Baud Rate | 64/76 | ⚠️ | ✅ | 4, 8 | 1 |
| Moving | 64/76 | ⚠️ | ✅ | 46, 49, 122, 570 | 1 |
| Present Temperature | 64/76 | ⚠️ | ✅ | 43, 46, 146, 594 | 1 |
| Shutdown | 64/76 | ⚠️ | ✅ | 18, 63 | 1 |
| Temperature Limit | 64/76 | ⚠️ | ✅ | 11, 12, 31 | 1 |
| Bus Watchdog | 63/76 | ⚠️ | ⚠️ | 8, 98, 546 | 1, 2 |
| Current Limit | 60/76 | ⚠️ | ✅ | 38, 66 | 2 |
| Goal Current | 60/76 | ⚠️ | ✅ | 102, 526, 550 | 2 |
| Present Current | 60/76 | ⚠️ | ✅ | 126, 546, 574 | 2 |
| Protocol Type | 60/76 | ⚠️ | ✅ | 11, 13 | 1 |
| LED | 58/76 | ⚠️ | ✅ | 25, 65, 513 | 1 |
| Secondary(Shadow) ID | 56/76 | ⚠️ | ✅ | 10, 12 | 1 |
| Hardware Error Status | 55/76 | ⚠️ | ✅ | 50, 70, 518 | 1 |
| Feedforward 1st Gain | 54/76 | ⚠️ | ✅ | 90, 538 | 2 |
| Feedforward 2nd Gain | 54/76 | ⚠️ | ✅ | 88, 536 | 2 |
| Acceleration Limit | 28/76 | ⚠️ | ✅ | 40, 68 | 4 |
| External Port Data 1 | 21/76 | ⚠️ | ✅ | 152, 600 | 2 |
| External Port Data 2 | 21/76 | ⚠️ | ✅ | 154, 602 | 2 |
| External Port Data 3 | 21/76 | ⚠️ | ✅ | 156, 604 | 2 |
| External Port Mode 1 | 21/76 | ✅ | ✅ | 56 | 1 |
| External Port Mode 2 | 21/76 | ✅ | ✅ | 57 | 1 |
| External Port Mode 3 | 21/76 | ✅ | ✅ | 58 | 1 |
| Backup Ready | 18/76 | ⚠️ | ✅ | 878, 919 | 1 |
| Startup Configuration | 18/76 | ⚠️ | ✅ | 34, 60 | 1 |
| Present Load | 16/76 | ⚠️ | ✅ | 40, 41, 126 | 2 |
| External Port Data 4 | 13/76 | ✅ | ✅ | 606 | 2 |
| External Port Mode 4 | 13/76 | ✅ | ✅ | 59 | 1 |
| LED Blue | 13/76 | ✅ | ✅ | 515 | 1 |
| LED Green | 13/76 | ✅ | ✅ | 514 | 1 |
| LED Red | 13/76 | ✅ | ✅ | 513 | 1 |
| Baud Rate (Bus) | 12/76 | ✅ | ✅ | 12 | 1 |
| Brake Delay | 12/76 | ✅ | ✅ | 106 | 2 |
| Controller State | 12/76 | ✅ | ✅ | 152 | 1 |
| Current Offset | 12/76 | ✅ | ✅ | 518 | 2 |
| Electronic GearRatio Denominator | 12/76 | ✅ | ✅ | 100 | 4 |
| Electronic GearRatio Numerator | 12/76 | ✅ | ✅ | 96 | 4 |
| Error Code | 12/76 | ✅ | ✅ | 153 | 1 |
| Error Code History 1 | 12/76 | ✅ | ✅ | 154 | 1 |
| Error Code History 10 | 12/76 | ✅ | ✅ | 163 | 1 |
| Error Code History 11 | 12/76 | ✅ | ✅ | 164 | 1 |
| Error Code History 12 | 12/76 | ✅ | ✅ | 165 | 1 |
| Error Code History 13 | 12/76 | ✅ | ✅ | 166 | 1 |
| Error Code History 14 | 12/76 | ✅ | ✅ | 167 | 1 |
| Error Code History 15 | 12/76 | ✅ | ✅ | 168 | 1 |
| Error Code History 16 | 12/76 | ✅ | ✅ | 169 | 1 |
| Error Code History 2 | 12/76 | ✅ | ✅ | 155 | 1 |
| Error Code History 3 | 12/76 | ✅ | ✅ | 156 | 1 |
| Error Code History 4 | 12/76 | ✅ | ✅ | 157 | 1 |
| Error Code History 5 | 12/76 | ✅ | ✅ | 158 | 1 |
| Error Code History 6 | 12/76 | ✅ | ✅ | 159 | 1 |
| Error Code History 7 | 12/76 | ✅ | ✅ | 160 | 1 |
| Error Code History 8 | 12/76 | ✅ | ✅ | 161 | 1 |
| Error Code History 9 | 12/76 | ✅ | ✅ | 162 | 1 |
| Following Error Threshold | 12/76 | ✅ | ✅ | 44 | 4 |
| Goal Current LPF Frequency | 12/76 | ✅ | ✅ | 134 | 2 |
| Goal Update Delay | 12/76 | ✅ | ✅ | 108 | 2 |
| Hybrid Save | 12/76 | ✅ | ✅ | 170 | 1 |
| In-Position Threshold | 12/76 | ✅ | ✅ | 40 | 4 |
| Inverter Temperature Limit | 12/76 | ✅ | ✅ | 56 | 1 |
| Motor Temperature Limit | 12/76 | ✅ | ✅ | 57 | 1 |
| Normal Excitation Voltage | 12/76 | ✅ | ✅ | 111 | 1 |
| Overexcitation Time | 12/76 | ✅ | ✅ | 112 | 2 |
| Overexcitation Voltage | 12/76 | ✅ | ✅ | 110 | 1 |
| PWM Offset | 12/76 | ✅ | ✅ | 516 | 2 |
| PWM Slope | 12/76 | ✅ | ✅ | 62 | 1 |
| Position FF Gain | 12/76 | ✅ | ✅ | 236 | 4 |
| Position FF LPF Time | 12/76 | ✅ | ✅ | 136 | 2 |
| Position Limit Threshold | 12/76 | ✅ | ✅ | 38 | 2 |
| Present Inverter Temperature | 12/76 | ✅ | ✅ | 570 | 1 |
| Present Motor Temperature | 12/76 | ✅ | ✅ | 571 | 1 |
| Present Velocity LPF Frequency | 12/76 | ✅ | ✅ | 132 | 2 |
| Profile Acceleration Time | 12/76 | ✅ | ✅ | 248 | 4 |
| Profile Time | 12/76 | ✅ | ✅ | 252 | 4 |
| Safe Stop Time | 12/76 | ✅ | ✅ | 104 | 2 |
| Velocity FF Gain | 12/76 | ✅ | ✅ | 220 | 4 |
| Velocity FF LPF Time | 12/76 | ✅ | ✅ | 138 | 2 |
| Velocity Offset | 12/76 | ✅ | ✅ | 520 | 4 |
| CCW Angle Limit | 10/76 | ✅ | ✅ | 8 | 2 |
| CW Angle Limit | 10/76 | ✅ | ✅ | 6 | 2 |
| Max Torque | 10/76 | ⚠️ | ✅ | 14, 15 | 2 |
| Moving Speed | 10/76 | ✅ | ✅ | 32 | 2 |
| Present Speed | 10/76 | ⚠️ | ✅ | 38, 39 | 2 |
| Present Voltage | 10/76 | ⚠️ | ✅ | 42, 45 | 1 |
| Punch | 10/76 | ⚠️ | ✅ | 48, 51 | 2 |
| Secondary ID | 10/76 | ✅ | ✅ | 12 | 1 |
| Torque Limit | 10/76 | ⚠️ | ✅ | 34, 35 | 2 |
| Alarm LED | 9/76 | ✅ | ✅ | 17 | 1 |
| Lock | 9/76 | ✅ | ✅ | 47 | 1 |
| Registered | 9/76 | ✅ | ✅ | 44 | 1 |
| CCW Compliance Margin | 6/76 | ✅ | ✅ | 27 | 1 |
| CCW Compliance Slope | 6/76 | ✅ | ✅ | 29 | 1 |
| CW Compliance Margin | 6/76 | ✅ | ✅ | 26 | 1 |
| CW Compliance Slope | 6/76 | ✅ | ✅ | 28 | 1 |
| D Gain | 4/76 | ⚠️ | ✅ | 26, 27 | 1 |
| I Gain | 4/76 | ⚠️ | ✅ | 27, 28 | 1 |
| P Gain | 4/76 | ⚠️ | ✅ | 28, 29 | 1 |
| BUS Watchdog | 3/76 | ✅ | ✅ | 98 | 1 |
| Goal Acceleration | 3/76 | ✅ | ✅ | 73 | 1 |
| Multi Turn Offset | 3/76 | ✅ | ✅ | 20 | 2 |
| Resolution Divider | 3/76 | ✅ | ✅ | 22 | 1 |
| Current | 2/76 | ✅ | ✅ | 68 | 2 |
| Goal Torque | 2/76 | ✅ | ✅ | 71 | 2 |
| Torque Ctrl Mode Enable | 2/76 | ✅ | ✅ | 70 | 1 |
| Control Mode | 1/76 | ✅ | ✅ | 11 | 1 |

## 4. Universal registers (present in ALL models)

**10** registers appear in every one of the 76 models.

### 4a. Universal AND uniform addr+size (1) — safe base-trait constants

| Register | Addr | Size |
|----------|:----:|:----:|
| Model Number | 0 | 2 |

### 4b. Universal but addr/size VARIES (9) — trait method, per-model address

| Register | Addr values | Size values |
|----------|-------------|-------------|
| Firmware Version | 2, 6 | 1 |
| Goal Position | 30, 116, 532, 564 | 2, 4 |
| ID | 3, 7 | 1 |
| Max Voltage Limit | 13, 14, 32, 60 | 1, 2 |
| Min Voltage Limit | 12, 13, 34, 62 | 1, 2 |
| Present Position | 36, 37, 132, 552, 580 | 2, 4 |
| Return Delay Time | 5, 9, 13 | 1 |
| Status Return Level | 15, 16, 17, 68, 516 | 1 |
| Torque Enable | 24, 64, 512 | 1 |

## 5. Registers whose address OR size is NOT constant

**66** registers cannot be a fixed constant — the trait must expose them as a per-model method (or they gate a capability). Full per-model breakdown:

### `Firmware Version` — 76/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 2 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28, xl320 |
| 6 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Goal Position` — 76/76 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 30 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28, xl320 |
| 116 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 532 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 564 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `ID` — 76/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 3 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28, xl320 |
| 7 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Max Voltage Limit` — 76/76 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 13 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 14 | 1 | xl320 |
| 32 | 2 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 60 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Min Voltage Limit` — 76/76 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 12 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 13 | 1 | xl320 |
| 34 | 2 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 62 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Present Position` — 76/76 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 36 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 37 | 2 | xl320 |
| 132 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 552 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 580 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Return Delay Time` — 76/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 5 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28, xl320 |
| 9 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 13 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Status Return Level` — 76/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 15 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 16 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 17 | 1 | xl320 |
| 68 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 516 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Torque Enable` — 76/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 24 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28, xl320 |
| 64 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 512 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Realtime Tick` — 69/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 50 | 2 | mx_106_p1, mx_28_p1, mx_64_p1 |
| 120 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 542 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 568 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Registered Instruction` — 67/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 16 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 47 | 1 | xl320 |
| 69 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 517 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Drive Mode` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 10 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_106_p1, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 32 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Goal PWM` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 100 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 524 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 548 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Goal Velocity` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 104 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 528 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 552 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Homing Offset` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 20 | 4 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 52 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Indirect Address 1` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 168 | 2 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 256 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Indirect Data 1` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 208 | 1 | xc330_m181_fw52, xc330_m288_fw52, xc330_t181_fw52, xc330_t288_fw52, xl330_m077_fw52, xl330_m288_fw52 |
| 224 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m288, xc330_t181, xc330_t288, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m288, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 634 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Max Position Limit` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 48 | 4 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 76 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Min Position Limit` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 52 | 4 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 84 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Moving Status` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 123 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 541 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 571 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Moving Threshold` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 24 | 4 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 48 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Operating Mode` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 11 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 33 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `PWM Limit` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 36 | 2 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 64 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Position D Gain` — 66/76 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 80 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 224 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 528 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Position I Gain` — 66/76 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 82 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 228 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 530 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Position P Gain` — 66/76 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 84 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 232 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 532 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Position Trajectory` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 140 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 560 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 588 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Present Input Voltage` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 144 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 568 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 592 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Present PWM` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 124 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 544 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 572 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Present Velocity` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 128 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 548 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 576 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Profile Acceleration` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 108 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 240 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 556 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Profile Velocity` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 112 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 244 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 560 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Velocity I Gain` — 66/76 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 76 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 212 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 524 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Velocity Limit` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 44 | 4 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 72 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Velocity P Gain` — 66/76 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 78 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 216 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 526 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Velocity Trajectory` — 66/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 136 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 564 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 584 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Baud Rate` — 64/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 4 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28, xl320 |
| 8 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `Moving` — 64/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 46 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 49 | 1 | xl320 |
| 122 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 570 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Present Temperature` — 64/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 43 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 46 | 1 | xl320 |
| 146 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 594 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Shutdown` — 64/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 18 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28, xl320 |
| 63 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `Temperature Limit` — 64/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 11 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 12 | 1 | xl320 |
| 31 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `Bus Watchdog` — 63/76 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 8 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 98 | 1 | 2xc430_w250, 2xl430_w250, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 546 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Current Limit` — 60/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 38 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 66 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Goal Current` — 60/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 102 | 2 | mx_106, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 526 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 550 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Present Current` — 60/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 126 | 2 | mx_106, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 546 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 574 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Protocol Type` — 60/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 11 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 13 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `LED` — 58/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 25 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28, xl320 |
| 65 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270 |
| 513 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Secondary(Shadow) ID` — 56/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 10 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 12 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `Hardware Error Status` — 55/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 50 | 1 | xl320 |
| 70 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 518 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Feedforward 1st Gain` — 54/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 90 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 538 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Feedforward 2nd Gain` — 54/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 88 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 536 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Acceleration Limit` — 28/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 40 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |
| 68 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `External Port Data 1` — 21/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 152 | 2 | xd540_t150, xd540_t270, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xm540_w150, xm540_w270 |
| 600 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `External Port Data 2` — 21/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 154 | 2 | xd540_t150, xd540_t270, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xm540_w150, xm540_w270 |
| 602 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `External Port Data 3` — 21/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 156 | 2 | xd540_t150, xd540_t270, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xm540_w150, xm540_w270 |
| 604 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Backup Ready` — 18/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 878 | 1 | ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250 |
| 919 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Startup Configuration` — 18/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 34 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 60 | 1 | ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250 |

### `Present Load` — 16/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 40 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 41 | 2 | xl320 |
| 126 | 2 | 2xc430_w250, 2xl430_w250, mx_28, xc430_w150, xc430_w240, xl430_w250 |

### `Max Torque` — 10/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 14 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 15 | 2 | xl320 |

### `Present Speed` — 10/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 38 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 39 | 2 | xl320 |

### `Present Voltage` — 10/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 42 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 45 | 1 | xl320 |

### `Punch` — 10/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 48 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 51 | 2 | xl320 |

### `Torque Limit` — 10/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 34 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, mx_106_p1, mx_28_p1, mx_64_p1, rx_28 |
| 35 | 2 | xl320 |

### `D Gain` — 4/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 26 | 1 | mx_106_p1, mx_28_p1, mx_64_p1 |
| 27 | 1 | xl320 |

### `I Gain` — 4/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 27 | 1 | mx_106_p1, mx_28_p1, mx_64_p1 |
| 28 | 1 | xl320 |

### `P Gain` — 4/76 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 28 | 1 | mx_106_p1, mx_28_p1, mx_64_p1 |
| 29 | 1 | xl320 |

## 6. Intra-model address aliases

Same address mapped to two+ register names inside one model (e.g. `Indirect Address 1` == `Indirect Address Write`). v2 must decide a canonical name per address.

_none_

## 7. Rare registers (present in ≤ 3 models) — model-specific extras

| Register | Models | Which |
|----------|:------:|-------|
| Control Mode | 1 | xl320 |
| Current | 2 | mx_106_p1, mx_64_p1 |
| Goal Torque | 2 | mx_106_p1, mx_64_p1 |
| Torque Ctrl Mode Enable | 2 | mx_106_p1, mx_64_p1 |
| BUS Watchdog | 3 | mx_106, mx_28, mx_64 |
| Goal Acceleration | 3 | mx_106_p1, mx_28_p1, mx_64_p1 |
| Multi Turn Offset | 3 | mx_106_p1, mx_28_p1, mx_64_p1 |
| Resolution Divider | 3 | mx_106_p1, mx_28_p1, mx_64_p1 |


# dxl_rs Control-Table Analysis

Generated from `control_tables/*.ron` — **73 models** (excludes `dynamixel.ron` model-number map).

- Protocol 1 only (`protocols: [P1]`): **6**
- Protocol 2 only (`protocols: [P2]`): **38**
- Both protocols (`protocols: [P1, P2]`): **29**
- No `protocols` yet: **0**
  - P1 only: ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28
  - P2 only: h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xl320, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099
  - P1 and P2: 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260
- Distinct register names across all models: **140**
- Distinct control-table layouts (exact signatures): **14**

## 1. Exact-duplicate groups

Models with **byte-identical control tables** (same register set, same address, same size for every register). One trait/struct can serve each whole group with zero per-model divergence.

| # | Proto | Members | Regs | Model #s |
|---|-------|---------|------|----------|
| 1 | P2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 | 98 | 4040, 4050, 4010, 4000, 4020, 4030, 4160, 4170, 4130, 4120, 4140, 4150 |
| 2 | P1, P2 | xd430_t210, xd430_t350, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xm430_w210, xm430_w350 | 57 | 1011, 1001, 1050, 1040, 1010, 1000, 1030, 1020 |
| 3 | P1, P2 | xd540_t150, xd540_t270, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xm540_w150, xm540_w270 | 63 | 1111, 1101, 1150, 1140, 1110, 1100, 1130, 1120 |
| 4 | P1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 | 32 | 12, 300, 18, 28 |
| 5 | P2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra | 67 | 51201, 53769, 54025, 43289, 46097, 46353 |
| 6 | P2 | ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250 | 70 | 2000, 2010, 2020, 2100, 2110, 2120 |
| 7 | P2 | xc330_m181, xc330_m288, xc330_t181, xc330_t288, xl330_m077, xl330_m288 | 58 | 1230, 1240, 1210, 1220, 1190, 1200 |
| 8 | P2 | xc330_m181_fw52, xc330_m288_fw52, xc330_t181_fw52, xc330_t288_fw52, xl330_m077_fw52, xl330_m288_fw52 | 58 | 1230, 1240, 1210, 1220, 1190, 1200 |
| 9 | P1, P2 | 2xc430_w250, 2xl430_w250, xc430_w150, xc430_w240, xl430_w250 | 55 | 1160, 1090, 1070, 1080, 1060 |
| 10 | P1, P2 | xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 | 56 | 1270, 1310, 1180, 1170 |
| 11 | P1, P2 | mx_106, mx_64 | 58 | 321, 311 |
| 12 | P1, P2 | mx_28 | 56 | 30 |
| 13 | P2 | rh_p12_rn | 63 | 35074 |
| 14 | P2 | xl320 | 31 | 350 |

## 2. Architectural tiers — the 4 RAM-layout generations

The 14 exact layouts collapse into **4 tiers** keyed by where `Goal Position` lives (the anchor that encodes the RAM-layout generation). Within a tier, tables share the same address space and differ only by which registers are present. Across tiers, even identically-named registers sit at different addresses — so a name-keyed trait works, but addresses must be per-model constants.

### T1 · Legacy layout (AX / RX-28 / XL320)
- 7 models, 2 exact layouts, Goal Position @ addr 30 size 2
- Register-count range: 31–32
- Common to all in tier: 25 regs · union 38
- Varies within tier (13): Alarm LED, CCW Compliance Margin, CCW Compliance Slope, CW Compliance Margin, CW Compliance Slope, Control Mode, D Gain, Hardware Error Status, I Gain, Lock, P Gain, Registered, Registered Instruction
- Layouts: [ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28] | [xl320]

### T2 · Modern X-series (XM/XH/XD/XC/XL430/XL330/XW + MX2.0)
- 41 models, 8 exact layouts, Goal Position @ addr 116 size 4
- Register-count range: 55–63
- Common to all in tier: 52 regs · union 67
- Varies within tier (15): Acceleration Limit, BUS Watchdog, Bus Watchdog, Current Limit, External Port Data 1, External Port Data 2, External Port Data 3, External Port Mode 1, External Port Mode 2, External Port Mode 3, Goal Current, LED, PWM Slope, Present Current, Present Load
- Layouts: [xd430_t210, xd430_t350, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xm430_w210, xm430_w350] | [xd540_t150, xd540_t270, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xm540_w150, xm540_w270] | [xc330_m181, xc330_m288, xc330_t181, xc330_t288, xl330_m077, xl330_m288] | [xc330_m181_fw52, xc330_m288_fw52, xc330_t181_fw52, xc330_t288_fw52, xl330_m077_fw52, xl330_m288_fw52] | [2xc430_w250, 2xl430_w250, xc430_w150, xc430_w240, xl430_w250] | [xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260] | [mx_106, mx_64] | [mx_28]

### T3 · YM series
- 12 models, 1 exact layouts, Goal Position @ addr 532 size 4
- Register-count range: 98–98
- Common to all in tier: 98 regs · union 98
- Layouts: [ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099]

### T4 · Pro / P-series (H/M/PH/PM/RH)
- 13 models, 3 exact layouts, Goal Position @ addr 564 size 4
- Register-count range: 63–70
- Common to all in tier: 62 regs · union 70
- Varies within tier (8): Backup Ready, Drive Mode, External Port Data 1, External Port Data 2, External Port Data 3, External Port Data 4, Protocol Type, Startup Configuration
- Layouts: [h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra] | [ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250] | [rh_p12_rn]

### 2b. Cross-layout similarity matrix

Upper cell = **register-name overlap** (Jaccard %). Lower cell = **address-identical overlap** — of the registers two layouts share by name, the % that also match on `(addr,size)`. High name-overlap with LOW address-overlap (e.g. X-series vs Pro) means: same concepts, different memory map → a trait can unify the API but not the constants.

Legend: **L1**=ax_12_plus(6m,P1) · **L2**=xl320(1m,P2) · **L3**=xd430_t210(8m,P1, P2) · **L4**=xd540_t150(8m,P1, P2) · **L5**=xc330_m181(6m,P2) · **L6**=xc330_m181_fw52(6m,P2) · **L7**=2xc430_w250(5m,P1, P2) · **L8**=xw430_t200(5m,P1, P2) · **L9**=mx_106(2m,P1, P2) · **L10**=mx_28(1m,P1, P2) · **L11**=ym070_210_a051(12m,P2) · **L12**=h42_20_s300_ra(6m,P2) · **L13**=ph42_020_s300(6m,P2) · **L14**=rh_p12_rn(1m,P2)

| name\addr | L1 | L2 | L3 | L4 | L5 | L6 | L7 | L8 | L9 | L10 | L11 | L12 | L13 | L14 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| L1 | **L1** | 66 | 22 | 20 | 22 | 22 | 24 | 21 | 22 | 24 | 9 | 18 | 17 | 19 |
| L2 | _48_ | **L2** | 26 | 24 | 25 | 25 | 28 | 24 | 25 | 28 | 10 | 21 | 20 | 22 |
| L3 | _6_ | _6_ | **L3** | 90 | 98 | 98 | 93 | 98 | 95 | 88 | 46 | 77 | 76 | 82 |
| L4 | _6_ | _6_ | _100_ | **L4** | 89 | 89 | 84 | 89 | 86 | 80 | 44 | 86 | 85 | 83 |
| L5 | _6_ | _6_ | _96_ | _96_ | **L5** | 100 | 92 | 97 | 93 | 87 | 46 | 76 | 75 | 81 |
| L6 | _6_ | _6_ | _93_ | _93_ | _95_ | **L6** | 92 | 97 | 93 | 87 | 46 | 76 | 75 | 81 |
| L7 | _6_ | _5_ | _100_ | _100_ | _96_ | _93_ | **L7** | 91 | 88 | 95 | 43 | 72 | 71 | 76 |
| L8 | _7_ | _6_ | _100_ | _100_ | _96_ | _93_ | _100_ | **L8** | 93 | 87 | 45 | 78 | 77 | 83 |
| L9 | _6_ | _6_ | _100_ | _100_ | _96_ | _93_ | _100_ | _100_ | **L9** | 93 | 46 | 76 | 75 | 81 |
| L10 | _6_ | _5_ | _100_ | _100_ | _96_ | _92_ | _100_ | _100_ | _100_ | **L10** | 43 | 71 | 70 | 75 |
| L11 | _9_ | _8_ | _8_ | _8_ | _8_ | _8_ | _9_ | _8_ | _8_ | _9_ | **L11** | 40 | 42 | 41 |
| L12 | _7_ | _6_ | _39_ | _40_ | _39_ | _39_ | _39_ | _39_ | _41_ | _41_ | _17_ | **L12** | 96 | 91 |
| L13 | _7_ | _6_ | _40_ | _41_ | _40_ | _40_ | _40_ | _40_ | _42_ | _42_ | _16_ | _100_ | **L13** | 90 |
| L14 | _7_ | _6_ | _39_ | _42_ | _39_ | _39_ | _39_ | _39_ | _41_ | _41_ | _17_ | _100_ | _100_ | **L14** |

## 3. Register universe — presence / address / size consistency

For every distinct register name: how many models have it, whether the **address** and **size** are the same in every model that has it, and the observed values. This is the core table for deciding what belongs in a shared trait vs a per-model override.

Legend: ✅ uniform · ⚠️ VARIES (address or size differs across models).

| Register | Models | Addr | Size | Addr values | Size values |
|----------|:------:|:----:|:----:|-------------|-------------|
| Firmware Version | 73/73 | ⚠️ | ✅ | 2, 6 | 1 |
| Goal Position | 73/73 | ⚠️ | ⚠️ | 30, 116, 532, 564 | 2, 4 |
| ID | 73/73 | ⚠️ | ✅ | 3, 7 | 1 |
| Max Voltage Limit | 73/73 | ⚠️ | ⚠️ | 13, 14, 32, 60 | 1, 2 |
| Min Voltage Limit | 73/73 | ⚠️ | ⚠️ | 12, 13, 34, 62 | 1, 2 |
| Model Number | 73/73 | ✅ | ✅ | 0 | 2 |
| Present Position | 73/73 | ⚠️ | ⚠️ | 36, 37, 132, 552, 580 | 2, 4 |
| Return Delay Time | 73/73 | ⚠️ | ✅ | 5, 9, 13 | 1 |
| Status Return Level | 73/73 | ⚠️ | ✅ | 15, 16, 17, 68, 516 | 1 |
| Torque Enable | 73/73 | ⚠️ | ✅ | 24, 64, 512 | 1 |
| Registered Instruction | 67/73 | ⚠️ | ✅ | 16, 47, 69, 517 | 1 |
| Goal PWM | 66/73 | ⚠️ | ✅ | 100, 524, 548 | 2 |
| Goal Velocity | 66/73 | ⚠️ | ✅ | 104, 528, 552 | 4 |
| Homing Offset | 66/73 | ⚠️ | ✅ | 20, 52 | 4 |
| Indirect Address 1 | 66/73 | ⚠️ | ✅ | 168, 256 | 2 |
| Indirect Address Read | 66/73 | ⚠️ | ✅ | 180, 296, 384, 578 | 2 |
| Indirect Address Write | 66/73 | ⚠️ | ✅ | 168, 256 | 2 |
| Indirect Data 1 | 66/73 | ⚠️ | ✅ | 208, 224, 634 | 1 |
| Indirect Data Read | 66/73 | ⚠️ | ✅ | 214, 230, 634, 698 | 1 |
| Indirect Data Write | 66/73 | ⚠️ | ✅ | 208, 224, 634 | 1 |
| Max Position Limit | 66/73 | ⚠️ | ✅ | 48, 76 | 4 |
| Min Position Limit | 66/73 | ⚠️ | ✅ | 52, 84 | 4 |
| Model Information | 66/73 | ✅ | ✅ | 2 | 4 |
| Moving Status | 66/73 | ⚠️ | ✅ | 123, 541, 571 | 1 |
| Moving Threshold | 66/73 | ⚠️ | ✅ | 24, 48 | 4 |
| Operating Mode | 66/73 | ⚠️ | ✅ | 11, 33 | 1 |
| PWM Limit | 66/73 | ⚠️ | ✅ | 36, 64 | 2 |
| Position D Gain | 66/73 | ⚠️ | ⚠️ | 80, 224, 528 | 2, 4 |
| Position I Gain | 66/73 | ⚠️ | ⚠️ | 82, 228, 530 | 2, 4 |
| Position P Gain | 66/73 | ⚠️ | ⚠️ | 84, 232, 532 | 2, 4 |
| Position Trajectory | 66/73 | ⚠️ | ✅ | 140, 560, 588 | 4 |
| Present Input Voltage | 66/73 | ⚠️ | ✅ | 144, 568, 592 | 2 |
| Present PWM | 66/73 | ⚠️ | ✅ | 124, 544, 572 | 2 |
| Present Velocity | 66/73 | ⚠️ | ✅ | 128, 548, 576 | 4 |
| Profile Acceleration | 66/73 | ⚠️ | ✅ | 108, 240, 556 | 4 |
| Profile Velocity | 66/73 | ⚠️ | ✅ | 112, 244, 560 | 4 |
| Realtime Tick | 66/73 | ⚠️ | ✅ | 120, 542, 568 | 2 |
| Velocity I Gain | 66/73 | ⚠️ | ⚠️ | 76, 212, 524 | 2, 4 |
| Velocity Limit | 66/73 | ⚠️ | ✅ | 44, 72 | 4 |
| Velocity P Gain | 66/73 | ⚠️ | ⚠️ | 78, 216, 526 | 2, 4 |
| Velocity Trajectory | 66/73 | ⚠️ | ✅ | 136, 564, 584 | 4 |
| Drive Mode | 65/73 | ⚠️ | ✅ | 10, 32 | 1 |
| Bus Watchdog | 63/73 | ⚠️ | ⚠️ | 8, 98, 546 | 1, 2 |
| Baud Rate | 61/73 | ⚠️ | ✅ | 4, 8 | 1 |
| Moving | 61/73 | ⚠️ | ✅ | 46, 49, 122, 570 | 1 |
| Present Temperature | 61/73 | ⚠️ | ✅ | 43, 46, 146, 594 | 1 |
| Shutdown | 61/73 | ⚠️ | ✅ | 18, 63 | 1 |
| Temperature Limit | 61/73 | ⚠️ | ✅ | 11, 12, 31 | 1 |
| Current Limit | 60/73 | ⚠️ | ✅ | 38, 66 | 2 |
| Goal Current | 60/73 | ⚠️ | ✅ | 102, 526, 550 | 2 |
| Present Current | 60/73 | ⚠️ | ✅ | 126, 546, 574 | 2 |
| Protocol Type | 60/73 | ⚠️ | ✅ | 11, 13 | 1 |
| Hardware Error Status | 55/73 | ⚠️ | ✅ | 50, 70, 518 | 1 |
| LED | 55/73 | ⚠️ | ✅ | 25, 65, 513 | 1 |
| Feedforward 1st Gain | 54/73 | ⚠️ | ✅ | 90, 538 | 2 |
| Feedforward 2nd Gain | 54/73 | ⚠️ | ✅ | 88, 536 | 2 |
| Secondary(Shadow) ID | 53/73 | ⚠️ | ✅ | 10, 12 | 1 |
| Acceleration Limit | 28/73 | ⚠️ | ✅ | 40, 68 | 4 |
| External Port Mode 1 | 21/73 | ✅ | ✅ | 56 | 1 |
| External Port Mode 2 | 21/73 | ✅ | ✅ | 57 | 1 |
| External Port Mode 3 | 21/73 | ✅ | ✅ | 58 | 1 |
| External Port Data 1 | 20/73 | ⚠️ | ✅ | 152, 600 | 2 |
| External Port Data 2 | 20/73 | ⚠️ | ✅ | 154, 602 | 2 |
| External Port Data 3 | 20/73 | ⚠️ | ✅ | 156, 604 | 2 |
| Backup Ready | 18/73 | ⚠️ | ✅ | 878, 919 | 1 |
| Startup Configuration | 18/73 | ⚠️ | ✅ | 34, 60 | 1 |
| External Port Mode 4 | 13/73 | ✅ | ✅ | 59 | 1 |
| LED Blue | 13/73 | ✅ | ✅ | 515 | 1 |
| LED Green | 13/73 | ✅ | ✅ | 514 | 1 |
| LED Red | 13/73 | ✅ | ✅ | 513 | 1 |
| Present Load | 13/73 | ⚠️ | ✅ | 40, 41, 126 | 2 |
| Secondary ID | 13/73 | ✅ | ✅ | 12 | 1 |
| Baud Rate (Bus) | 12/73 | ✅ | ✅ | 12 | 1 |
| Brake Delay | 12/73 | ✅ | ✅ | 106 | 2 |
| Controller State | 12/73 | ✅ | ✅ | 152 | 1 |
| Current Offset | 12/73 | ✅ | ✅ | 518 | 2 |
| Electronic GearRatio Denominator | 12/73 | ✅ | ✅ | 100 | 4 |
| Electronic GearRatio Numerator | 12/73 | ✅ | ✅ | 96 | 4 |
| Error Code | 12/73 | ✅ | ✅ | 153 | 1 |
| Error Code History 1 | 12/73 | ✅ | ✅ | 154 | 1 |
| Error Code History 10 | 12/73 | ✅ | ✅ | 163 | 1 |
| Error Code History 11 | 12/73 | ✅ | ✅ | 164 | 1 |
| Error Code History 12 | 12/73 | ✅ | ✅ | 165 | 1 |
| Error Code History 13 | 12/73 | ✅ | ✅ | 166 | 1 |
| Error Code History 14 | 12/73 | ✅ | ✅ | 167 | 1 |
| Error Code History 15 | 12/73 | ✅ | ✅ | 168 | 1 |
| Error Code History 16 | 12/73 | ✅ | ✅ | 169 | 1 |
| Error Code History 2 | 12/73 | ✅ | ✅ | 155 | 1 |
| Error Code History 3 | 12/73 | ✅ | ✅ | 156 | 1 |
| Error Code History 4 | 12/73 | ✅ | ✅ | 157 | 1 |
| Error Code History 5 | 12/73 | ✅ | ✅ | 158 | 1 |
| Error Code History 6 | 12/73 | ✅ | ✅ | 159 | 1 |
| Error Code History 7 | 12/73 | ✅ | ✅ | 160 | 1 |
| Error Code History 8 | 12/73 | ✅ | ✅ | 161 | 1 |
| Error Code History 9 | 12/73 | ✅ | ✅ | 162 | 1 |
| External Port Data 4 | 12/73 | ✅ | ✅ | 606 | 2 |
| Following Error Threshold | 12/73 | ✅ | ✅ | 44 | 4 |
| Goal Current LPF Frequency | 12/73 | ✅ | ✅ | 134 | 2 |
| Goal Update Delay | 12/73 | ✅ | ✅ | 108 | 2 |
| Hybrid Save | 12/73 | ✅ | ✅ | 170 | 1 |
| In-Position Threshold | 12/73 | ✅ | ✅ | 40 | 4 |
| Inverter Temperature Limit | 12/73 | ✅ | ✅ | 56 | 1 |
| Motor Temperature Limit | 12/73 | ✅ | ✅ | 57 | 1 |
| Normal Excitation Voltage | 12/73 | ✅ | ✅ | 111 | 1 |
| Overexcitation Time | 12/73 | ✅ | ✅ | 112 | 2 |
| Overexcitation Voltage | 12/73 | ✅ | ✅ | 110 | 1 |
| PWM Offset | 12/73 | ✅ | ✅ | 516 | 2 |
| PWM Slope | 12/73 | ✅ | ✅ | 62 | 1 |
| Position FF Gain | 12/73 | ✅ | ✅ | 236 | 4 |
| Position FF LPF Time | 12/73 | ✅ | ✅ | 136 | 2 |
| Position Limit Threshold | 12/73 | ✅ | ✅ | 38 | 2 |
| Present Inverter Temperature | 12/73 | ✅ | ✅ | 570 | 1 |
| Present Motor Temperature | 12/73 | ✅ | ✅ | 571 | 1 |
| Present Velocity LPF Frequency | 12/73 | ✅ | ✅ | 132 | 2 |
| Profile Acceleration Time | 12/73 | ✅ | ✅ | 248 | 4 |
| Profile Time | 12/73 | ✅ | ✅ | 252 | 4 |
| Safe Stop Time | 12/73 | ✅ | ✅ | 104 | 2 |
| Velocity FF Gain | 12/73 | ✅ | ✅ | 220 | 4 |
| Velocity FF LPF Time | 12/73 | ✅ | ✅ | 138 | 2 |
| Velocity Offset | 12/73 | ✅ | ✅ | 520 | 4 |
| CCW Angle Limit | 7/73 | ✅ | ✅ | 8 | 2 |
| CW Angle Limit | 7/73 | ✅ | ✅ | 6 | 2 |
| Max Torque | 7/73 | ⚠️ | ✅ | 14, 15 | 2 |
| Moving Speed | 7/73 | ✅ | ✅ | 32 | 2 |
| Present Speed | 7/73 | ⚠️ | ✅ | 38, 39 | 2 |
| Present Voltage | 7/73 | ⚠️ | ✅ | 42, 45 | 1 |
| Punch | 7/73 | ⚠️ | ✅ | 48, 51 | 2 |
| Torque Limit | 7/73 | ⚠️ | ✅ | 34, 35 | 2 |
| Alarm LED | 6/73 | ✅ | ✅ | 17 | 1 |
| CCW Compliance Margin | 6/73 | ✅ | ✅ | 27 | 1 |
| CCW Compliance Slope | 6/73 | ✅ | ✅ | 29 | 1 |
| CW Compliance Margin | 6/73 | ✅ | ✅ | 26 | 1 |
| CW Compliance Slope | 6/73 | ✅ | ✅ | 28 | 1 |
| Lock | 6/73 | ✅ | ✅ | 47 | 1 |
| Registered | 6/73 | ✅ | ✅ | 44 | 1 |
| BUS Watchdog | 3/73 | ✅ | ✅ | 98 | 1 |
| Control Mode | 1/73 | ✅ | ✅ | 11 | 1 |
| D Gain | 1/73 | ✅ | ✅ | 27 | 1 |
| I Gain | 1/73 | ✅ | ✅ | 28 | 1 |
| P Gain | 1/73 | ✅ | ✅ | 29 | 1 |

## 4. Universal registers (present in ALL models)

**10** registers appear in every one of the 73 models.

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

**67** registers cannot be a fixed constant — the trait must expose them as a per-model method (or they gate a capability). Full per-model breakdown:

### `Firmware Version` — 73/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 2 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28, xl320 |
| 6 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Goal Position` — 73/73 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 30 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28, xl320 |
| 116 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 532 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 564 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `ID` — 73/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 3 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28, xl320 |
| 7 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Max Voltage Limit` — 73/73 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 13 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 14 | 1 | xl320 |
| 32 | 2 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 60 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Min Voltage Limit` — 73/73 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 12 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 13 | 1 | xl320 |
| 34 | 2 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 62 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Present Position` — 73/73 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 36 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 37 | 2 | xl320 |
| 132 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 552 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 580 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Return Delay Time` — 73/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 5 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28, xl320 |
| 9 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 13 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Status Return Level` — 73/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 15 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 16 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 17 | 1 | xl320 |
| 68 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 516 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Torque Enable` — 73/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 24 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28, xl320 |
| 64 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 512 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Registered Instruction` — 67/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 16 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 47 | 1 | xl320 |
| 69 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 517 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Goal PWM` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 100 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 524 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 548 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Goal Velocity` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 104 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 528 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 552 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Homing Offset` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 20 | 4 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 52 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Indirect Address 1` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 168 | 2 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 256 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Indirect Address Read` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 180 | 2 | xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52 |
| 296 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |
| 384 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 578 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `Indirect Address Write` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 168 | 2 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 256 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Indirect Data 1` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 208 | 1 | xc330_m181_fw52, xc330_m288_fw52, xc330_t181_fw52, xc330_t288_fw52, xl330_m077_fw52, xl330_m288_fw52 |
| 224 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m288, xc330_t181, xc330_t288, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m288, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 634 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Indirect Data Read` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 214 | 1 | xc330_m181_fw52, xc330_m288_fw52, xc330_t181_fw52, xc330_t288_fw52, xl330_m077_fw52, xl330_m288_fw52 |
| 230 | 1 | xc330_m181, xc330_m288, xc330_t181, xc330_t288, xl330_m077, xl330_m288 |
| 634 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 698 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Indirect Data Write` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 208 | 1 | xc330_m181_fw52, xc330_m288_fw52, xc330_t181_fw52, xc330_t288_fw52, xl330_m077_fw52, xl330_m288_fw52 |
| 224 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m288, xc330_t181, xc330_t288, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m288, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 634 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Max Position Limit` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 48 | 4 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 76 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Min Position Limit` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 52 | 4 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 84 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Moving Status` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 123 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 541 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 571 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Moving Threshold` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 24 | 4 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 48 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Operating Mode` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 11 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 33 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `PWM Limit` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 36 | 2 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 64 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Position D Gain` — 66/73 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 80 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 224 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 528 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Position I Gain` — 66/73 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 82 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 228 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 530 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Position P Gain` — 66/73 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 84 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 232 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 532 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Position Trajectory` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 140 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 560 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 588 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Present Input Voltage` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 144 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 568 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 592 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Present PWM` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 124 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 544 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 572 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Present Velocity` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 128 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 548 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 576 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Profile Acceleration` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 108 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 240 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 556 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Profile Velocity` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 112 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 244 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 560 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Realtime Tick` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 120 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 542 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 568 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Velocity I Gain` — 66/73 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 76 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 212 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 524 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Velocity Limit` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 44 | 4 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 72 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Velocity P Gain` — 66/73 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 78 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 216 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 526 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Velocity Trajectory` — 66/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 136 | 4 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 564 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 584 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Drive Mode` — 65/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 10 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 32 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Bus Watchdog` — 63/73 models  ·  ⚠️ address varies  ·  ⚠️ size varies
| addr | size | models |
|:----:|:----:|--------|
| 8 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 98 | 1 | 2xc430_w250, 2xl430_w250, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 546 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Baud Rate` — 61/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 4 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28, xl320 |
| 8 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `Moving` — 61/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 46 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 49 | 1 | xl320 |
| 122 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 570 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Present Temperature` — 61/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 43 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 46 | 1 | xl320 |
| 146 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 594 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Shutdown` — 61/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 18 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28, xl320 |
| 63 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `Temperature Limit` — 61/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 11 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 12 | 1 | xl320 |
| 31 | 1 | 2xc430_w250, 2xl430_w250, h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `Current Limit` — 60/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 38 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 66 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Goal Current` — 60/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 102 | 2 | mx_106, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 526 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 550 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Present Current` — 60/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 126 | 2 | mx_106, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 546 | 2 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 574 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Protocol Type` — 60/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 11 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 13 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `Hardware Error Status` — 55/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 50 | 1 | xl320 |
| 70 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 518 | 1 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `LED` — 55/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 25 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28, xl320 |
| 65 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270 |
| 513 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Feedforward 1st Gain` — 54/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 90 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 538 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Feedforward 2nd Gain` — 54/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 88 | 2 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |
| 536 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |

### `Secondary(Shadow) ID` — 53/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 10 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 12 | 1 | 2xc430_w250, 2xl430_w250, mx_106, mx_28, mx_64, xc330_m181, xc330_m181_fw52, xc330_m288, xc330_m288_fw52, xc330_t181, xc330_t181_fw52, xc330_t288, xc330_t288_fw52, xc430_w150, xc430_w240, xd430_t210, xd430_t350, xd540_t150, xd540_t270, xh430_v210, xh430_v350, xh430_w210, xh430_w350, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xl330_m077, xl330_m077_fw52, xl330_m288, xl330_m288_fw52, xl430_w250, xm430_w210, xm430_w350, xm540_w150, xm540_w270, xw430_t200, xw430_t333, xw540_h260, xw540_t140, xw540_t260 |

### `Acceleration Limit` — 28/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 40 | 4 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, mx_106, mx_28, mx_64, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250, rh_p12_rn |
| 68 | 4 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `External Port Data 1` — 20/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 152 | 2 | xd540_t150, xd540_t270, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xm540_w150, xm540_w270 |
| 600 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250 |

### `External Port Data 2` — 20/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 154 | 2 | xd540_t150, xd540_t270, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xm540_w150, xm540_w270 |
| 602 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250 |

### `External Port Data 3` — 20/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 156 | 2 | xd540_t150, xd540_t270, xh540_v150, xh540_v270, xh540_w150, xh540_w270, xm540_w150, xm540_w270 |
| 604 | 2 | h42_20_s300_ra, h54_100_s500_ra, h54_200_s500_ra, m42_10_s260_ra, m54_40_s250_ra, m54_60_s250_ra, ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250 |

### `Backup Ready` — 18/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 878 | 1 | ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250 |
| 919 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |

### `Startup Configuration` — 18/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 34 | 1 | ym070_210_a051, ym070_210_a099, ym070_210_b001, ym070_210_m001, ym070_210_r051, ym070_210_r099, ym080_230_a051, ym080_230_a099, ym080_230_b001, ym080_230_m001, ym080_230_r051, ym080_230_r099 |
| 60 | 1 | ph42_020_s300, ph54_100_s500, ph54_200_s500, pm42_010_s260, pm54_040_s250, pm54_060_s250 |

### `Present Load` — 13/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 40 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 41 | 2 | xl320 |
| 126 | 2 | 2xc430_w250, 2xl430_w250, mx_28, xc430_w150, xc430_w240, xl430_w250 |

### `Max Torque` — 7/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 14 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 15 | 2 | xl320 |

### `Present Speed` — 7/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 38 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 39 | 2 | xl320 |

### `Present Voltage` — 7/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 42 | 1 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 45 | 1 | xl320 |

### `Punch` — 7/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 48 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 51 | 2 | xl320 |

### `Torque Limit` — 7/73 models  ·  ⚠️ address varies
| addr | size | models |
|:----:|:----:|--------|
| 34 | 2 | ax_12_plus, ax_12a, ax_12w, ax_18a, ax_18f, rx_28 |
| 35 | 2 | xl320 |

## 6. Intra-model address aliases

Same address mapped to two+ register names inside one model (e.g. `Indirect Address 1` == `Indirect Address Write`). v2 must decide a canonical name per address.

- **2xc430_w250** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **2xl430_w250** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **h42_20_s300_ra** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **h54_100_s500_ra** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **h54_200_s500_ra** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **m42_10_s260_ra** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **m54_40_s250_ra** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **m54_60_s250_ra** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **mx_106** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **mx_28** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **mx_64** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **ph42_020_s300** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ph54_100_s500** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ph54_200_s500** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **pm42_010_s260** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **pm54_040_s250** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **pm54_060_s250** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **rh_p12_rn** — @168: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **xc330_m181** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xc330_m181_fw52** — @168: Indirect Address 1 == Indirect Address Write; @208: Indirect Data 1 == Indirect Data Write
- **xc330_m288** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xc330_m288_fw52** — @168: Indirect Address 1 == Indirect Address Write; @208: Indirect Data 1 == Indirect Data Write
- **xc330_t181** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xc330_t181_fw52** — @168: Indirect Address 1 == Indirect Address Write; @208: Indirect Data 1 == Indirect Data Write
- **xc330_t288** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xc330_t288_fw52** — @168: Indirect Address 1 == Indirect Address Write; @208: Indirect Data 1 == Indirect Data Write
- **xc430_w150** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xc430_w240** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xd430_t210** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xd430_t350** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xd540_t150** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xd540_t270** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xh430_v210** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xh430_v350** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xh430_w210** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xh430_w350** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xh540_v150** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xh540_v270** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xh540_w150** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xh540_w270** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xl330_m077** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xl330_m077_fw52** — @168: Indirect Address 1 == Indirect Address Write; @208: Indirect Data 1 == Indirect Data Write
- **xl330_m288** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xl330_m288_fw52** — @168: Indirect Address 1 == Indirect Address Write; @208: Indirect Data 1 == Indirect Data Write
- **xl430_w250** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xm430_w210** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xm430_w350** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xm540_w150** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xm540_w270** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xw430_t200** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xw430_t333** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xw540_h260** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xw540_t140** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **xw540_t260** — @168: Indirect Address 1 == Indirect Address Write; @224: Indirect Data 1 == Indirect Data Write
- **ym070_210_a051** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym070_210_a099** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym070_210_b001** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym070_210_m001** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym070_210_r051** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym070_210_r099** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym080_230_a051** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym080_230_a099** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym080_230_b001** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym080_230_m001** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym080_230_r051** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write
- **ym080_230_r099** — @256: Indirect Address 1 == Indirect Address Write; @634: Indirect Data 1 == Indirect Data Write

## 7. Rare registers (present in ≤ 3 models) — model-specific extras

| Register | Models | Which |
|----------|:------:|-------|
| Control Mode | 1 | xl320 |
| D Gain | 1 | xl320 |
| I Gain | 1 | xl320 |
| P Gain | 1 | xl320 |
| BUS Watchdog | 3 | mx_106, mx_28, mx_64 |


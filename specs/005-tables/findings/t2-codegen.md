# T080: generated diff, each file with its cause (2026-10-29)

`cargo xtask codegen` in the T2 worktree: `generated 75 files under src/`, no skip warning.
Base: `origin/integration` (`7dd7469`), whose `src/models*` were generated from T1's tables by X1.
Every changed line below traces to a `control_tables/**` change listed in a T2 findings file; no
change without a cause, nothing edited by hand (constitution V).

| Files | Change | Cause (findings) |
|---|---|---|
| `src/models/mx_28_p1.rs`, `mx_64_p1.rs`, `mx_106_p1.rs` (new) | types `Mx28P1`, `Mx64P1`, `Mx106P1`: P1 only, `ModeSource::AngleLimits`, CW/CCW limits, scale 2048 / 0 / 4095; `Mx64P1` doc "Verified against …/mx/mx-64/ on 2026-10-29", the other two "Unverified" | new MX P1 tables (`t2-mx_64_p1.md`, `t2-mx_28_p1.md`, `t2-mx_106_p1.md`), map 29 / 310 / 320 (`t2-map.md`) |
| `src/models.rs` | `mod`/`pub use` for the three types; `ActuatorKind::{Mx28P1, Mx64P1, Mx106P1}` and their match arms; lookup arms `(P1, 29)`, `(P1, 310)`, `(P1, 320)` (and the `not(feature = "mx")` `UnsupportedModel` arms); variant docs "also sold as AX-12+ / AX-18F / XW430-T200"; "RH-P12-RN(A)" | map (`t2-map.md`) |
| `src/models/tests.rs` | generated lookup, concrete-match and describe tests for 29, 310, 320; `(P2, 29/310/320)` in `no_tables_on_this_protocol`; `(P1, 29/310/320)` in `mx_off` | map (`t2-map.md`) |
| `src/models/mx_64.rs` | 4 alias constants removed; doc "Verified against <https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/> on 2026-10-29" | `t2-mx_64.md` |
| 59 files: `mx_28`, `mx_106`; X430 `two_xc430_w250`, `two_xl430_w250`, `xc430_w150`, `xc430_w240`, `xd430_t210`, `xd430_t350`, `xh430_v210`, `xh430_v350`, `xh430_w210`, `xh430_w350`, `xl430_w250`, `xm430_w210`, `xm430_w350`; X540 (11); X330 (12, `_fw52` included); PRO (A) (6) and PRO+ PM (3); Y (12) | only the 4 constants `INDIRECT_ADDRESS_WRITE`, `INDIRECT_DATA_WRITE`, `INDIRECT_ADDRESS_READ`, `INDIRECT_DATA_READ` removed | alias rows (`t2-mx.md`, `t2-x430.md`, `t2-x540.md`, `t2-x330.md`, `t2-pro.md`, `t2-y.md`) |
| `src/models/xw430_t333.rs` | 4 alias constants removed; doc "also sold as XW430-T200" | `t2-x430.md`, `t2-map.md` |
| `src/models/ph42_020_s300.rs`, `ph54_100_s500.rs`, `ph54_200_s500.rs` | 4 alias constants removed; `SECONDARY_ID` → `SECONDARY_SHADOW_ID` (12, 1, RW, EEPROM) | `t2-pro.md` |
| `src/models/rh_p12_rn.rs` | 4 alias constants removed; `EXTERNAL_PORT_DATA_1` … `_4` (600–606, 2, RW, RAM) added; name and doc "RH-P12-RN(A)" | `t2-pro.md`, `t2-map.md` |
| `src/models/ax_12a.rs`, `ax_18a.rs` | doc "also sold as AX-12+" / "AX-18F" | `t2-map.md` |

Totals: 260 alias constants removed (65 mapped types × 4); 3 new types; 3 renamed constants; 4
added constants. No `ModeValues` changed (no `operating_modes` list changed and no mode row was
added). 72 files changed in all (`git diff --stat`: 619 insertions, 299 deletions); every other
generated file (`ax_12w`, `rx_28`, `xl320`, …) is unchanged. `ax_12_plus`, `ax_18f`, `xw430_t200`
generate nothing (not mapped).

Analysis regenerated: `python3 control_tables/analyze_tables.py` → `models=76 exact_layouts=18
tiers=4 regs=142 universal=10 varies=66` (`control_table_analysis.md` and `.json`; FR-015).

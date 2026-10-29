# T075, T079: US5 T2 slice (2026-10-29)

## T075

`test_check_tables.py` `MapTests.test_also_is_a_non_empty_list_of_names` (failed first, 5
sub-failures): `also: "AX-12+"` (not a list), `also: []`, `also: [""]`, `also: [12]` and a repeated
name → exit 1 naming the model number and `also`; `also: ["AX-12+"]` and `["AX-12+", "AX-12"]` →
0; a P1-only entry without `default` → 0 (US5 AS1). Rule added to `check_map` in
`check_tables.py`. (Repeated names are a tables-only addition; X1 reads `also` as
`Vec<String>` and accepts anything.)

## T079

- `python3 control_tables/check_tables.py`: exit 0,
  `summary: 76 tables, 5 verified, 71 unverified, 73 mapped, 3 not mapped, 28 dual-protocol, 0 errors`.
  This is the target the caller re-derived for RX-28 (the task list's original target predates
  RX-28); no difference.
- Entries 29, 310, 320: `p1` only (`mx_28_p1` 29 p1, `mx_64_p1` 310 p1, `mx_106_p1` 320 p1).
- `grep -L '"Operating Mode"' control_tables/*.ron`: `ax_12_plus`, `ax_12a`, `ax_12w`, `ax_18a`,
  `ax_18f`, `xl320`, `mx_28_p1`, `mx_64_p1`, `mx_106_p1`, `dynamixel.ron` and **`rx_28`** (RX-28,
  verified in T1, has the CW/CCW pair; the task text predates it).
- `grep -L 'operating_modes' control_tables/*.ron`: the same files but `xl320`.
- `cargo xtask codegen`: no skip warning (`ax_12_plus`, `ax_18f`, `xw430_t200` skipped silently,
  named in `also`: "AX-12A (model number 12, also sold as AX-12+)", "AX-18A (… also sold as
  AX-18F)", "XW430-T333 (… also sold as XW430-T200)"); generates `Mx28P1`, `Mx64P1`, `Mx106P1`.
  `RhP12Rn` is now documented and named "RH-P12-RN(A)".

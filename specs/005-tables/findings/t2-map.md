# T2 model map changes (`control_tables/dynamixel.ron`)

## T062: 310 MX-64 (Protocol 1 firmware)

- Added `310: (family: "mx", name: "MX-64", p1: [(from_fw: 0, file: "mx_64_p1")])`, after 30
  and before 311. Page https://emanual.robotis.com/docs/en/dxl/mx/mx-64/, fetched 2026-10-29:
  Model Number row Initial Value **310**; title "MX-64T/R/AT/AR" (name "MX-64" as research R-11
  and the contract example). `p1` only, no `default` (one list).
- It was written with the legacy `tables: [(from_fw: 0, file: "mx_64_p1")]` as well, because the
  T1 rule ("legacy `tables` equals the only list") was still in force until T073; T073 removes
  every `tables` list, this one included.

## T072–T074: legacy `tables` removed and rejected

- T072: `test_check_tables.py` `MapTests.test_legacy_tables_equals_the_default_list` replaced by
  `test_tables_key_is_rejected` (a `tables` key on a dual-protocol entry, on a P1-only entry, or
  empty → exit 1 naming the model number and `tables`; an entry without it passes). The `entry()`
  helper no longer writes `tables` by default, and the literal fixture entries lost theirs. The
  new test failed first (30 failures: every fixture now lacked the T1-required key).
- T073: `check_tables.py` reports "the legacy `tables` key was removed in T2; use `p1`/`p2`"
  for any entry with `tables`; `tables` no longer counts as a map list (`LIST_KEYS = ("p1",
  "p2")`). `dynamixel.ron`: `tables: [...]` deleted from all 65 entries, and the `tables:`
  paragraph removed from the header comment. `analyze_tables.py` reads only `p1`/`p2`.
- T074: `check_tables.py` exit 0 (`74 tables, 5 verified, 69 unverified, 71 mapped, 3 not
  mapped, 28 dual-protocol, 0 errors`); unit tests OK; `grep -c 'tables:' control_tables/dynamixel.ron`
  is 0; `cargo xtask codegen` → `0 written, 0 removed, 73 unchanged`: the removal changed no
  generated file (X1 ignores `tables`; US4 AS5, FR-009).

## T078: MX Protocol 1 entries, `also`, RH-P12-RN(A)

Each value read on the cited page, fetched 2026-10-29:

| Change | Page | Evidence |
|---|---|---|
| add `29: (family: "mx", name: "MX-28", p1: [(from_fw: 0, file: "mx_28_p1")])` | https://emanual.robotis.com/docs/en/dxl/mx/mx-28/ | Model Number Initial Value 29; title "MX-28T/R/AT/AR" |
| add `320: (family: "mx", name: "MX-106", p1: [(from_fw: 0, file: "mx_106_p1")])` | https://emanual.robotis.com/docs/en/dxl/mx/mx-106/ | Model Number Initial Value 320; body "MX-106R, MX-106T" |
| `also: ["AX-12+"]` on 12 | https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ | heading "AX-12+, AX-12A"; Model Number 12 |
| `also: ["AX-18F"]` on 18 | https://emanual.robotis.com/docs/en/dxl/ax/ax-18a/ | heading "AX-18F, AX-18A"; Model Number 18 |
| `also: ["XW430-T200"]` on 1270 | https://emanual.robotis.com/docs/en/dxl/x/xw430-t200/ | Model Number "1,270" (heading "XW430-T200-R"); confirmed again (first 2026-10-28) |
| 35074 `name: "RH-P12-RN"` → `"RH-P12-RN(A)"` | https://emanual.robotis.com/docs/en/platform/rh_p12_rna/ | title "RH-P12-RN(A)"; Model Number "35,074"; "RH-P12-RN(A) uses different Control Table from RH-P12-RN" |

`ax_12_plus`, `ax_18f`, `xw430_t200` stay in `not_mapped` with `SameNumberAs(12)`, `(18)`,
`(1270)` (US5 AS4, FR-012). 29, 310, 320 have `p1` only and no `default`.

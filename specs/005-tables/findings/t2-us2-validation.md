# T063: US2 T2 slice validation (2026-10-29)

- `python3 control_tables/check_tables.py`: exit 0,
  `summary: 74 tables, 5 verified, 69 unverified, 71 mapped, 3 not mapped, 28 dual-protocol, 0 errors`.
  Verified: `ax_12a` (2026-10-28), `rx_28` (2026-10-28), `mx_64`, `ax_12_plus`, `mx_64_p1`
  (2026-10-29). No mode-source or `operating_modes` error. Y-series and PRO tables still mapped
  and `unverified` (US2 AS5, FR-013).
- `python3 control_tables/emanual.py diff`: `ax_12_plus` 0 differences (exit 0); `mx_64_p1` 0
  differences (exit 0); `mx_64` exit 1 with exactly 22 "only on page" indirect rows (Indirect
  Address/Data 2, 3, 26–31, 54–56 and 29), which T060 / research R-9 say not to add (question
  in `t2-mx_64.md`). Mode rows: `mx_64` `Operating Mode` (11, 1, RW, EEPROM) matches its page;
  `operating_modes` `[3]`, `[4, 5]`, `[0, 1, 16]` matches the page (SC-008); `ax_12_plus` and
  `mx_64_p1` have no mode row, as their pages.
- SC-002 audits: 5 random rows each, recorded in `t2-mx_64.md`, `t2-ax_12_plus.md`,
  `t2-mx_64_p1.md` (15 of 15 match).
- `python3 control_tables/check_tables.py --markdown mx_64 ax_12_plus mx_64_p1`:

```text
- `mx_64`: verified https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/ 2026-10-29, [P1, P2], 311 p1 p2
- `ax_12_plus`: verified https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ 2026-10-29, [P1], not mapped: same number as 12
- `mx_64_p1`: verified https://emanual.robotis.com/docs/en/dxl/mx/mx-64/ 2026-10-29, [P1], 310 p1
```

- `cargo xtask codegen`: `generated 73 files under src/ (4 written, 0 removed, 69 unchanged)`.
  Diff: new `src/models/mx_64_p1.rs` (`Mx64P1`, model 310, P1, `ModeSource::AngleLimits`, verified
  doc); `src/models.rs` (module, re-export, `ActuatorKind::Mx64P1`, `(P1, 310)` lookup arms and
  the match arms); `src/models/mx_64.rs` (the four alias constants removed; doc line "Verified
  against …/mx-64-2/ on 2026-10-29"); `src/models/tests.rs` (generated tests for 310). No
  `ModeValues` change in `Mx64::DATA` (T060 corrected no list). `ax_12_plus` is not mapped, so
  its verification changes no generated file.

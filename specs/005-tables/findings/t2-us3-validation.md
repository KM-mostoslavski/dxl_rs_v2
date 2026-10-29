# T071: US3 T2 slice validation (2026-10-29)

- `python3 control_tables/check_tables.py`: exit 0,
  `summary: 74 tables, 5 verified, 69 unverified, 71 mapped, 3 not mapped, 28 dual-protocol, 0 errors`
  (no role or mode row removed; every mode row with its `operating_modes`).
- `grep -l 'Indirect Address Write' control_tables/*.ron`: empty. No page lists that name (nor
  "Indirect Data Write", "Indirect Address Read", "Indirect Data Read"); the four names are gone
  from all 66 tables that carried them (65 in this phase plus `mx_64` in T060).
- `grep -L '"Operating Mode"' control_tables/*.ron`: the 5 `ax_*` tables, `rx_28`, `xl320`,
  `mx_64_p1`, `dynamixel.ron` (no mode row added, moved or removed in this phase: no page showed a
  mode-row difference).
- `cargo xtask codegen`: `generated 73 files under src/ (64 written, 0 removed, 9 unchanged)`.
  Diff reviewed (each item listed in a T2 findings file):
  - 260 alias constants removed (`INDIRECT_ADDRESS_WRITE`, `INDIRECT_DATA_WRITE`,
    `INDIRECT_ADDRESS_READ`, `INDIRECT_DATA_READ` × 65 mapped types: MX(2.0) 3, X430 14, X540 11,
    X330 12, PRO/PRO+/RH 13, Y 12; `xw430_t200` is not mapped) — `t2-mx_64.md`, `t2-mx.md`,
    `t2-x430.md`, `t2-x540.md`, `t2-x330.md`, `t2-pro.md`, `t2-y.md`.
  - `SECONDARY_ID` → `SECONDARY_SHADOW_ID` in `Ph42020S300`, `Ph54100S500`, `Ph54200S500` — `t2-pro.md`.
  - `EXTERNAL_PORT_DATA_1` … `_4` added to `RhP12Rn` — `t2-pro.md`.
  - No `ModeValues` change (no mode row added in this phase).
  - Plus the phase 10 changes (`Mx64P1`, 310 lookup and tests, `Mx64` verified doc) —
    `t2-us2-validation.md`.
- `cargo check --all-targets`: builds; no non-generated code used a removed constant.

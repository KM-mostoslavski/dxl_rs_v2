# T1 PRO / PRO+ / RH group (T033): STOPPED, raised to the architect

Files: `h42_20_s300_ra`, `h54_100_s500_ra`, `h54_200_s500_ra`, `m42_10_s260_ra`, `m54_40_s250_ra`,
`m54_60_s250_ra`, `ph42_020_s300`, `ph54_100_s500`, `ph54_200_s500`, `pm42_010_s260`,
`pm54_040_s250`, `pm54_060_s250`, `rh_p12_rn`. Pages from `PAGES` (PRO (A): `…/dxl/pro/<model>-ra/`;
PRO+: `…/dxl/p/<model>-r/`; RH-P12-RN(A): `https://emanual.robotis.com/docs/en/platform/rh_p12_rna/`),
all fetched 2026-10-28.

## Why the task stopped

All 13 pages list an access value other than `R` or `RW`: **`R/RW`** on External Port Data 1–4
(addresses 600, 602, 604, 606; size 2; "Control Table of RAM Area"). T028's rule (applied by T033):
"any other access value stops the task and is raised"; the `emanual.py fill` contract: "It stops
with exit code 1, writing nothing, when the page has an access value other than `R` or `RW`".
`python3 control_tables/emanual.py diff <stem>` and `fill <stem>` exit 1 on every page:

```
error: row 600 'External Port Data 1': access 'R/RW' is neither R nor RW
```

The page explains the value (PH54-200-S500-R, "External Port Mode, External Port Data (56, 57, 58,
59)"): "The property of each port is configured by the External Port Mode (56 ~ 59) and data of
external port is controlled by the External Port Data(600 ~ 607)." Its function table gives the
External Port Data access per mode: Analogue Input (mode 0) `R`, Digital Output Push-Pull (mode 1)
`W`, and further modes. So the access depends on a run-time setting.

Twelve tables carry the four External Port Data rows; `rh_p12_rn` does not, but its page lists them
too, so the page-level stop applies to it as well.

## What was written

Nothing beyond T015's `source: Unverified`. No `protocols`, no `access`/`area`, no `type_info`
change, no mode-row confirmation. The check therefore fails on these 13 tables (missing
`protocols`, rows without `access`/`area`, and, from T027, the missing `operating_modes`).

## Question for the architect (blocks T033, T040, and T1's check)

The schema allows `access: R | RW` only (ron-schema §1, "anything else is a contract change"). How
should External Port Data 1–4 be recorded: (a) a third access value (contract change in
module-interfaces §3.1 and core's `Register`), (b) `RW` (the widest access the page lists for the
register), or (c) another rule? Once decided, T033 runs as written (fill, protocols, type_info,
mode rows), then T040.

Expected from research, not yet confirmed per page (the task stopped first): PRO+ and RH-P12-RN(A)
Protocol Type 2, 10 (Modbus-RTU) → `[P2]`; PRO (A) per the page statement; mode row `Operating Mode`
(11, 1) on every page.

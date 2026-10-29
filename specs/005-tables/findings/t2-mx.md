# T2 MX(2.0) rest (T064): `mx_28`, `mx_106`

T1 findings: `t1-mx-xl320.md` (only the four alias rows per table; no other row finding, no
mode-row finding). `mx_64` was done in T060. Both tables stay `source: Unverified`.

Rule (research R-14): "Indirect Address Write", "Indirect Data Write", "Indirect Address Read",
"Indirect Data Read" are removed only where the page does not list that name. Page rows read with
`emanual.py` (`PAGES`), diffed with `emanual.compare`, before and after. No row added (the
numbered indirect rows only on the page stay out, as in T060). Mode row: `Operating Mode` (11,
1, RW, EEPROM) on both pages, as the tables; `operating_modes` unchanged.

Generated effect: `INDIRECT_ADDRESS_WRITE`, `INDIRECT_DATA_WRITE`, `INDIRECT_ADDRESS_READ`,
`INDIRECT_DATA_READ` leave `Mx28` and `Mx106`.

### `mx_28`

Page https://emanual.robotis.com/docs/en/dxl/mx/mx-28-2/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 224 1 "Indirect Data Write" (not on the page).
- Removed 578 2 "Indirect Address Read" (not on the page).
- Removed 634 1 "Indirect Data Read" (not on the page).
- Diff before: 24 differences; only in table (0) | only on page (20): 170 2 "Indirect Address 2" RW RAM; 172 2 "Indirect Address 3" RW RAM; 218 2 "Indirect Address 26" RW RAM; 220 2 "Indirect Address 27" RW RAM; 222 2 "Indirect Address 28" RW RAM; 225 1 "Indirect Data 2" RW RAM; 226 1 "Indirect Data 3" RW RAM; 249 1 "Indirect Data 26" RW RAM; 250 1 "Indirect Data 27" RW RAM; 251 1 "Indirect Data 28" RW RAM; 580 2 "Indirect Address 30" RW RAM; 582 2 "Indirect Address 31" RW RAM; 628 2 "Indirect Address 54" RW RAM; 630 2 "Indirect Address 55" RW RAM; 632 2 "Indirect Address 56" RW RAM; 635 1 "Indirect Data 30" RW RAM; 636 1 "Indirect Data 31" RW RAM; 659 1 "Indirect Data 54" RW RAM; 660 1 "Indirect Data 55" RW RAM; 661 1 "Indirect Data 56" RW RAM | different name (4): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 224 1 table "Indirect Data Write", page "Indirect Data 1"; 578 2 table "Indirect Address Read", page "Indirect Address 29"; 634 1 table "Indirect Data Read", page "Indirect Data 29" | different access/area (0)
- Diff after: 22 differences; only in table (0) | only on page (22): 170 2 "Indirect Address 2" RW RAM; 172 2 "Indirect Address 3" RW RAM; 218 2 "Indirect Address 26" RW RAM; 220 2 "Indirect Address 27" RW RAM; 222 2 "Indirect Address 28" RW RAM; 225 1 "Indirect Data 2" RW RAM; 226 1 "Indirect Data 3" RW RAM; 249 1 "Indirect Data 26" RW RAM; 250 1 "Indirect Data 27" RW RAM; 251 1 "Indirect Data 28" RW RAM; 578 2 "Indirect Address 29" RW RAM; 580 2 "Indirect Address 30" RW RAM; 582 2 "Indirect Address 31" RW RAM; 628 2 "Indirect Address 54" RW RAM; 630 2 "Indirect Address 55" RW RAM; 632 2 "Indirect Address 56" RW RAM; 634 1 "Indirect Data 29" RW RAM; 635 1 "Indirect Data 30" RW RAM; 636 1 "Indirect Data 31" RW RAM; 659 1 "Indirect Data 54" RW RAM; 660 1 "Indirect Data 55" RW RAM; 661 1 "Indirect Data 56" RW RAM | different name (0) | different access/area (0)

### `mx_106`

Page https://emanual.robotis.com/docs/en/dxl/mx/mx-106-2/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 224 1 "Indirect Data Write" (not on the page).
- Removed 578 2 "Indirect Address Read" (not on the page).
- Removed 634 1 "Indirect Data Read" (not on the page).
- Diff before: 24 differences; only in table (0) | only on page (20): 170 2 "Indirect Address 2" RW RAM; 172 2 "Indirect Address 3" RW RAM; 218 2 "Indirect Address 26" RW RAM; 220 2 "Indirect Address 27" RW RAM; 222 2 "Indirect Address 28" RW RAM; 225 1 "Indirect Data 2" RW RAM; 226 1 "Indirect Data 3" RW RAM; 249 1 "Indirect Data 26" RW RAM; 250 1 "Indirect Data 27" RW RAM; 251 1 "Indirect Data 28" RW RAM; 580 2 "Indirect Address 30" RW RAM; 582 2 "Indirect Address 31" RW RAM; 628 2 "Indirect Address 54" RW RAM; 630 2 "Indirect Address 55" RW RAM; 632 2 "Indirect Address 56" RW RAM; 635 1 "Indirect Data 30" RW RAM; 636 1 "Indirect Data 31" RW RAM; 659 1 "Indirect Data 54" RW RAM; 660 1 "Indirect Data 55" RW RAM; 661 1 "Indirect Data 56" RW RAM | different name (4): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 224 1 table "Indirect Data Write", page "Indirect Data 1"; 578 2 table "Indirect Address Read", page "Indirect Address 29"; 634 1 table "Indirect Data Read", page "Indirect Data 29" | different access/area (0)
- Diff after: 22 differences; only in table (0) | only on page (22): 170 2 "Indirect Address 2" RW RAM; 172 2 "Indirect Address 3" RW RAM; 218 2 "Indirect Address 26" RW RAM; 220 2 "Indirect Address 27" RW RAM; 222 2 "Indirect Address 28" RW RAM; 225 1 "Indirect Data 2" RW RAM; 226 1 "Indirect Data 3" RW RAM; 249 1 "Indirect Data 26" RW RAM; 250 1 "Indirect Data 27" RW RAM; 251 1 "Indirect Data 28" RW RAM; 578 2 "Indirect Address 29" RW RAM; 580 2 "Indirect Address 30" RW RAM; 582 2 "Indirect Address 31" RW RAM; 628 2 "Indirect Address 54" RW RAM; 630 2 "Indirect Address 55" RW RAM; 632 2 "Indirect Address 56" RW RAM; 634 1 "Indirect Data 29" RW RAM; 635 1 "Indirect Data 30" RW RAM; 636 1 "Indirect Data 31" RW RAM; 659 1 "Indirect Data 54" RW RAM; 660 1 "Indirect Data 55" RW RAM; 661 1 "Indirect Data 56" RW RAM | different name (0) | different access/area (0)


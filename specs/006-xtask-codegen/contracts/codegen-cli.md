# Contract: `cargo xtask codegen`

**Feature**: [../spec.md](../spec.md) | **Model**: [../data-model.md](../data-model.md) |
**Input schema**: `specs/ideas/module-interfaces.md` §3 (owned by tables)

## Invocation

```sh
cargo xtask codegen        # alias in .cargo/config.toml: run --quiet --package xtask --
```

No other arguments. Any other first argument prints `usage: cargo xtask codegen` to stderr and
exits with 1.

## Reads

- `control_tables/dynamixel.ron`, with the RON `IMPLICIT_SOME` extension.
- Every table it names, as `control_tables/<file>.ron` with braces rewritten to parentheses.
- The names of all `control_tables/*.ron` files, for the unmapped-table warning.

Field rules are in [data-model.md §1](../data-model.md#1-input-xtask-codegenschemars). Unknown
fields are ignored.

## Writes

Only `src/models.rs` and `src/models/*.rs`. A file whose new contents equal the old bytes is
not rewritten. A `.rs` file in `src/models/` that is no longer generated is deleted. Nothing is
written unless parsing, every check, rendering and formatting all succeeded.

## Output streams and exit status

| Outcome | stdout | stderr | Exit |
|---|---|---|---|
| Success | `generated <n> files under src/ (<w> written, <r> removed, <u> unchanged)` | one `warning:` line per unmapped table | 0 |
| Check failures | nothing | every `error:` line, sorted, then the warnings | 1 |
| `rustfmt` missing | nothing | ``error: rustfmt not found: run `rustup component add rustfmt` `` | 1 |
| `rustfmt` failed on a file | nothing | `error: rustfmt failed on src/models/<file>.rs: <rustfmt stderr>` | 1 |
| I/O error while writing | nothing | `error: <path>: <io error>` | 1 |

Line formats:

```text
error: control_tables/<file>.ron: <problem>
error: control_tables/dynamixel.ron: model <n>: <problem>
warning: control_tables/<file>.ron: not in dynamixel.ron (models or not_mapped), skipped
```

Messages name the role, register, protocol, address or family involved. Examples (wording is not
contract; tests match on file plus key words):

```text
error: control_tables/xm430_w350.ron: missing role Goal Position (row "Goal Position")
error: control_tables/ax_12a.ron: role Torque Enable must be RW/RAM, found R/RAM
error: control_tables/ph54_200_s500.ron: listed under p1 but Goal Position is at address 564 (> 255)
error: control_tables/dynamixel.ron: model 1190: p2 from_fw must strictly increase
error: control_tables/dynamixel.ron: model 1020: unknown family "x_series" (expected ax, mx, x-series, y-series, pro)
```

## Guarantees

- Same inputs and the same rustfmt give byte-identical output (FR-002, SC-001).
- One row's change in one table changes exactly one generated file (SC-007). Exception: the
  Model Number row, which every table must agree on, so changing it in one table is a check
  failure.
- On any `error:` line, every file under `src/models*` keeps its bytes and its modification time
  (FR-003, SC-006).

# Contract: feature-set list

**Feature**: [../spec.md](../spec.md) | **Model**: [../data-model.md](../data-model.md) |
**Research**: [../research.md § R-1, R-2](../research.md#r-1-how-the-feature-sets-are-listed)

The `feature-sets` job turns `Cargo.toml` into the lists below. The build legs, the test legs,
the no_std job and the compile-fail job read them. The rule and the output shape are the contract; the exact `jq` text is not.

## Input

`cargo metadata --no-deps --format-version 1`, package `dxl_rs_v2`, field `.features`: a map
from feature name to the list of entries it enables.

## Rule

1. `declared` = every key of `.features` except `default`, sorted by name.
2. `closure(f)` = `f` plus, recursively, every entry of `f` that has no `:` and no `/`.
   (`dep:x` and `x/y` name dependencies, not local features.)
3. `needs_std(f)` = `std` ∈ `closure(f)`. `needs_alloc(f)` = `alloc` ∈ `closure(f)`.
4. Host sets: `no features`, then `<f> alone` for every `f` in `declared`, then `defaults`,
   then `all features`. Count = `|declared| + 3` (SC-005).
5. Target sets (`thumbv7em-none-eabihf`): `<f> alone` for every `f` in `declared` with
   `not needs_std(f)`.
6. `heap_free` = comma list of `declared` with neither `needs_std` nor `needs_alloc`.
7. `no_std` = comma list of `declared` without `needs_std`.
8. Host test sets: `std` (`--no-default-features --features std`), then `<f>` for every `f` in
   `heap_free` (`--no-default-features --features std,<f>`). They run `cargo test --lib`, so
   crate tests that compile only with a family off (e.g. generated `src/models/tests.rs`) run.

## Output (job outputs, compact JSON)

```text
matrix:     [ Leg, ... ]          # host legs first, then target legs
tests:      [ TestLeg, ... ]      # "std" first, then one per heap_free feature
heap_free:  "ax,mock,mx,pro,x-series,y-series"
no_std:     "alloc,ax,mock,mx,pro,x-series,y-series"

Leg = {
  "name":      string,   # "no features" | "<feature> alone" | "defaults" | "all features"
  "where":     string,   # "host" | "thumbv7em"
  "args":      string,   # cargo flags, e.g. "--no-default-features --features ax"
                         #   target legs end with "--target thumbv7em-none-eabihf"
  "needs_std": boolean   # true: the leg installs libudev-dev first (Linux serialport)
}

TestLeg = {
  "name": string,        # "std" | "<feature>"
  "args": string         # "--no-default-features --features std" | "... --features std,<feature>"
}
```

Leg check name: `feature set (<where>): <name>`, e.g. `feature set (host): std alone`,
`feature set (thumbv7em): mock alone`. Test leg check name: `feature tests (host): <name>`, e.g.
`feature tests (host): ax`. Test legs always enable `std`, so they always install libudev-dev.

## Expected output for `Cargo.toml` on `main` (PR #7 merged)

| where | sets |
|---|---|
| host (11) | no features; alloc, ax, mock, mx, pro, std, x-series, y-series alone; defaults; all features |
| thumbv7em (7) | alloc, ax, mock, mx, pro, x-series, y-series alone |
| tests, host (7) | std; ax, mock, mx, pro, x-series, y-series |

`needs_std` is true for `std alone`, `defaults` and `all features`.

Prototype (scratch copies of PR #7 and `main`, `-D warnings`): all 7 test sets pass
`cargo test --lib`.

After bus-5 declares `partial-reads = []` and `raw-registers = []`: 13 host, 9 target and 9 test
sets; both features join `heap_free` and `no_std`, so the no_std job and the std-off
compile-fail pass build them too. No workflow edit.

## Reference program (prototyped, not normative)

```jq
def closure($f): . as $n
  | [$n] + [ ($f[$n] // [])[] | select(test("[:/]") | not) | closure($f)[] ];
.packages[] | select(.name == $pkg) | .features as $f
| [ $f | keys[] | select(. != "default") ] as $all
| def needs($x): closure($f) | any(.[]; . == $x);
{ host:      ( [ {name: "no features", args: "--no-default-features"} ]
             + [ $all[] | {name: "\(.) alone", args: "--no-default-features --features \(.)"} ]
             + [ {name: "defaults", args: ""}, {name: "all features", args: "--all-features"} ] ),
  target:    [ $all[] | select(needs("std") | not)
             | {name: "\(.) alone", args: "--no-default-features --features \(.)"} ],
  heap_free: ( [ $all[] | select((needs("std") or needs("alloc")) | not) ] | join(",") ),
  tests:     ( [ {name: "std", args: "--no-default-features --features std"} ]
             + [ $all[] | select((needs("std") or needs("alloc")) | not)
               | {name: ., args: "--no-default-features --features std,\(.)"} ] ),
  no_std:    ( [ $all[] | select(needs("std") | not) ] | join(",") ) }
```

The workflow version adds `where`, `needs_std` and the `--target` flag, and merges `host` and
`target` into one `matrix` array.

## Guarantees

- Adding, removing or renaming a feature in `Cargo.toml` changes the lists with no workflow edit.
- A feature that needs neither `std` nor `alloc` always gets a test leg.
- A feature that enables `std`, directly or through another feature, never gets a target leg.
- Failure: if `cargo metadata` or `jq` fails, the `feature-sets` job fails, every build and
  test leg is skipped, and the `feature sets` aggregate check is red. It never passes with an empty list.

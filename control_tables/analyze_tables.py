#!/usr/bin/env python3
"""Exhaustive control-table analysis for dxl_rs v2 API design.

Parses every docs/control_table_ron/*.ron, extracts the control_table
(address,size,data_name) triples, and computes:
  - exact-duplicate groups (identical tables)
  - per-register presence/address/size consistency across all models
  - address collisions (aliases) within a model
  - pairwise similarity / clustering (exact -> similar -> unique)
  - protocol classification (P1 vs P2 by CW/CCW Angle Limit presence)

Emits a big markdown report to control_table_analysis.md.
"""
import re, os, json, itertools
from collections import defaultdict, OrderedDict

RON_DIR = os.path.dirname(os.path.abspath(__file__))
MODEL_MAP_FILE = os.path.join(RON_DIR, "dynamixel.ron")
OUT = os.path.join(RON_DIR, "control_table_analysis.md")

# address: N, size: N, data_name: "..."  (order fixed in these RON files)
ENTRY = re.compile(r'address:\s*(\d+)\s*,\s*size:\s*(\d+)\s*,\s*data_name:\s*"([^"]+)"')

SKIP = {"dynamixel.ron"}  # model-number map, not a control table

def load_model_numbers():
    m = {}
    try:
        txt = open(MODEL_MAP_FILE).read()
    except OSError:
        return m
    for num, fn in re.findall(r'(\d+)\s*:\s*"([^"]+)\.model"', txt):
        m[fn] = int(num)
    return m

def parse(path):
    txt = open(path).read()
    # entries: list of (name, addr, size) in file order (may contain alias dups)
    entries = []
    for a, s, name in ENTRY.findall(txt):
        entries.append((name, int(a), int(s)))
    return entries

def main():
    model_num = load_model_numbers()
    files = sorted(f for f in os.listdir(RON_DIR)
                   if f.endswith(".ron") and f not in SKIP)

    models = OrderedDict()   # stem -> dict
    for f in files:
        entries = parse(os.path.join(RON_DIR, f))
        if not entries:
            continue
        stem = f[:-4]
        # register map keyed by name -> (addr,size); track alias collisions on addr
        regmap = OrderedDict()
        addr_names = defaultdict(list)   # addr -> [names]
        for name, addr, size in entries:
            addr_names[addr].append(name)
            if name not in regmap:
                regmap[name] = (addr, size)
        aliases = {a: ns for a, ns in addr_names.items() if len(set(ns)) > 1}
        is_p1 = ("CW Angle Limit" in regmap and "CCW Angle Limit" in regmap)
        models[stem] = dict(
            file=f, entries=entries, regmap=regmap, aliases=aliases,
            proto="P1" if is_p1 else "P2",
            model_number=model_num.get(f[:-4] + ".model"),
        )

    # ---- exact-duplicate groups: signature over sorted (name,addr,size) ----
    sig_groups = defaultdict(list)
    for stem, m in models.items():
        sig = tuple(sorted((n, a, s) for n, (a, s) in m["regmap"].items()))
        sig_groups[sig].append(stem)
    exact_groups = sorted(sig_groups.values(), key=lambda g: (-len(g), g[0]))

    # ---- register universe: name -> per-model (addr,size) ----
    reg_universe = defaultdict(dict)  # name -> {stem: (addr,size)}
    for stem, m in models.items():
        for n, (a, s) in m["regmap"].items():
            reg_universe[n][stem] = (a, s)

    n_models = len(models)
    reg_rows = []
    for name, per in reg_universe.items():
        present = len(per)
        addrs = set(v[0] for v in per.values())
        sizes = set(v[1] for v in per.values())
        reg_rows.append(dict(
            name=name, present=present,
            addr_uniform=len(addrs) == 1, size_uniform=len(sizes) == 1,
            addrs=sorted(addrs), sizes=sorted(sizes), per=per,
        ))
    # sort: universal & uniform first, then by presence desc, then name
    reg_rows.sort(key=lambda r: (-r["present"], r["name"]))

    # ---- pairwise similarity over exact-layout representatives ----
    stems = list(models.keys())
    nameset = {s: set(models[s]["regmap"].keys()) for s in stems}
    reps = [g[0] for g in exact_groups]          # 1 rep per exact layout
    rep_size = {g[0]: len(g) for g in exact_groups}

    def name_jaccard(a, b):
        A, B = nameset[a], nameset[b]
        return len(A & B) / len(A | B) if (A | B) else 1.0

    def addr_overlap(a, b):
        """Fraction of SHARED register names that sit at the SAME (addr,size).
        This is the real 'drop-in / same-trait-impl' metric — name overlap
        alone lies (position is 'Goal Position' in both X and Pro, but at 116
        vs 564)."""
        A, B = models[a]["regmap"], models[b]["regmap"]
        shared = set(A) & set(B)
        if not shared:
            return 0.0
        same = sum(1 for n in shared if A[n] == B[n])
        return same / len(shared)

    # ---- architectural tier: keyed by (proto, Goal Position addr,size) ----
    # The Goal-Position location encodes the RAM-layout 'generation'.
    def tier_key(stem):
        gp = models[stem]["regmap"].get("Goal Position")
        return (models[stem]["proto"], gp)
    TIER_LABEL = {
        ("P1", (30, 2)): "T1 · P1 legacy (AX / XL320)",
        ("P2", (116, 4)): "T2 · Modern X-series (XM/XH/XD/XC/XL430/XL330/XW + MX2.0)",
        ("P2", (532, 4)): "T3 · YM series",
        ("P2", (564, 4)): "T4 · Pro / P-series (H/M/PH/PM/RH)",
    }
    tiers = defaultdict(list)
    for g in exact_groups:
        tiers[tier_key(g[0])].append(g)

    # ---------------------------------------------------------------- report
    L = []
    def w(s=""): L.append(s)

    w("# dxl_rs Control-Table Analysis")
    w()
    w(f"Generated from `docs/control_table_ron/*.ron` — **{n_models} models** "
      f"(excludes `dynamixel.ron` model-number map).")
    w()
    w(f"- Protocol 1 (CW/CCW Angle Limit): "
      f"**{sum(1 for m in models.values() if m['proto']=='P1')}**")
    w(f"- Protocol 2: "
      f"**{sum(1 for m in models.values() if m['proto']=='P2')}**")
    w(f"- Distinct register names across all models: **{len(reg_universe)}**")
    w(f"- Distinct control-table layouts (exact signatures): **{len(exact_groups)}**")
    w()

    # ========== 1. EXACT-DUPLICATE GROUPS ==========
    w("## 1. Exact-duplicate groups")
    w()
    w("Models with **byte-identical control tables** (same register set, same "
      "address, same size for every register). One trait/struct can serve each "
      "whole group with zero per-model divergence.")
    w()
    w("| # | Proto | Members | Regs | Model #s |")
    w("|---|-------|---------|------|----------|")
    for i, g in enumerate(exact_groups, 1):
        rep = models[g[0]]
        nums = ", ".join(str(models[s]["model_number"]) for s in g
                         if models[s]["model_number"] is not None)
        w(f"| {i} | {rep['proto']} | {', '.join(g)} | "
          f"{len(rep['regmap'])} | {nums or '—'} |")
    w()

    # ========== 2. ARCHITECTURAL TIERS (similar) ==========
    w("## 2. Architectural tiers — the 4 RAM-layout generations")
    w()
    w("The 14 exact layouts collapse into **4 tiers** keyed by where `Goal "
      "Position` lives (the anchor that encodes the RAM-layout generation). "
      "Within a tier, tables share the same address space and differ only by "
      "which registers are present. Across tiers, even identically-named "
      "registers sit at different addresses — so a name-keyed trait works, but "
      "addresses must be per-model constants.")
    w()
    tier_order = [("P1", (30, 2)), ("P2", (116, 4)),
                  ("P2", (532, 4)), ("P2", (564, 4))]
    tier_list = [(k, tiers[k]) for k in tier_order if k in tiers] + \
                [(k, v) for k, v in tiers.items() if k not in tier_order]
    for tk, gs in tier_list:
        members = [s for g in gs for s in g]
        common = set.intersection(*(nameset[s] for s in members))
        union = set.union(*(nameset[s] for s in members))
        label = TIER_LABEL.get(tk, f"{tk[0]} · Goal Position @ {tk[1]}")
        w(f"### {label}")
        w(f"- {len(members)} models, {len(gs)} exact layouts, "
          f"Goal Position @ addr {tk[1][0]} size {tk[1][1]}")
        w(f"- Register-count range: "
          f"{min(len(models[s]['regmap']) for s in members)}–"
          f"{max(len(models[s]['regmap']) for s in members)}")
        w(f"- Common to all in tier: {len(common)} regs · union {len(union)}")
        if len(gs) > 1:
            w(f"- Varies within tier ({len(union - common)}): "
              f"{', '.join(sorted(union - common))}")
        w(f"- Layouts: " + " | ".join(
            f"[{', '.join(g)}]" for g in sorted(gs, key=lambda g: -len(g))))
        w()

    # ---- 14x14 similarity matrix over exact-layout reps ----
    w("### 2b. Cross-layout similarity matrix")
    w()
    w("Upper cell = **register-name overlap** (Jaccard %). Lower cell = "
      "**address-identical overlap** — of the registers two layouts share by "
      "name, the % that also match on `(addr,size)`. High name-overlap with LOW "
      "address-overlap (e.g. X-series vs Pro) means: same concepts, different "
      "memory map → a trait can unify the API but not the constants.")
    w()
    # order reps by tier then size
    def rep_sort(r):
        tk = tier_key(r)
        return (tier_order.index(tk) if tk in tier_order else 99, -rep_size[r])
    ordr = sorted(reps, key=rep_sort)
    tags = {r: f"L{i+1}" for i, r in enumerate(ordr)}
    w("Legend: " + " · ".join(
        f"**{tags[r]}**={r}({rep_size[r]}m,{models[r]['proto']})" for r in ordr))
    w()
    header = "| name\\addr | " + " | ".join(tags[r] for r in ordr) + " |"
    w(header)
    w("|" + "---|" * (len(ordr) + 1))
    for a in ordr:
        cells = []
        for b in ordr:
            if a == b:
                cells.append(f"**{tags[a]}**")
            elif ordr.index(b) > ordr.index(a):
                cells.append(f"{round(name_jaccard(a,b)*100)}")   # upper: names
            else:
                cells.append(f"_{round(addr_overlap(a,b)*100)}_")  # lower: addr
        w(f"| {tags[a]} | " + " | ".join(cells) + " |")
    w()

    # ========== 3. REGISTER MATRIX ==========
    w("## 3. Register universe — presence / address / size consistency")
    w()
    w("For every distinct register name: how many models have it, whether the "
      "**address** and **size** are the same in every model that has it, and the "
      "observed values. This is the core table for deciding what belongs in a "
      "shared trait vs a per-model override.")
    w()
    w("Legend: ✅ uniform · ⚠️ VARIES (address or size differs across models).")
    w()
    w("| Register | Models | Addr | Size | Addr values | Size values |")
    w("|----------|:------:|:----:|:----:|-------------|-------------|")
    for r in reg_rows:
        au = "✅" if r["addr_uniform"] else "⚠️"
        su = "✅" if r["size_uniform"] else "⚠️"
        av = str(r["addrs"][0]) if r["addr_uniform"] else ", ".join(map(str, r["addrs"]))
        sv = str(r["sizes"][0]) if r["size_uniform"] else ", ".join(map(str, r["sizes"]))
        w(f"| {r['name']} | {r['present']}/{n_models} | {au} | {su} | {av} | {sv} |")
    w()

    # ========== 4. UNIVERSAL REGISTERS ==========
    universal = [r for r in reg_rows if r["present"] == n_models]
    uni_uniform = [r for r in universal if r["addr_uniform"] and r["size_uniform"]]
    uni_varies = [r for r in universal if not (r["addr_uniform"] and r["size_uniform"])]
    w("## 4. Universal registers (present in ALL models)")
    w()
    w(f"**{len(universal)}** registers appear in every one of the {n_models} models.")
    w()
    w(f"### 4a. Universal AND uniform addr+size ({len(uni_uniform)}) — "
      f"safe base-trait constants")
    w()
    if uni_uniform:
        w("| Register | Addr | Size |")
        w("|----------|:----:|:----:|")
        for r in uni_uniform:
            w(f"| {r['name']} | {r['addrs'][0]} | {r['sizes'][0]} |")
    else:
        w("_none_")
    w()
    w(f"### 4b. Universal but addr/size VARIES ({len(uni_varies)}) — "
      f"trait method, per-model address")
    w()
    if uni_varies:
        w("| Register | Addr values | Size values |")
        w("|----------|-------------|-------------|")
        for r in uni_varies:
            w(f"| {r['name']} | {', '.join(map(str, r['addrs']))} | "
              f"{', '.join(map(str, r['sizes']))} |")
    else:
        w("_none_")
    w()

    # ========== 5. VARIES REGISTERS (the hard ones) ==========
    varies = [r for r in reg_rows if not (r["addr_uniform"] and r["size_uniform"])]
    w("## 5. Registers whose address OR size is NOT constant")
    w()
    w(f"**{len(varies)}** registers cannot be a fixed constant — the trait must "
      f"expose them as a per-model method (or they gate a capability). Full "
      f"per-model breakdown:")
    w()
    for r in varies:
        w(f"### `{r['name']}` — {r['present']}/{n_models} models"
          + ("" if r["addr_uniform"] else "  ·  ⚠️ address varies")
          + ("" if r["size_uniform"] else "  ·  ⚠️ size varies"))
        # group models by (addr,size)
        by_val = defaultdict(list)
        for stem, (a, s) in r["per"].items():
            by_val[(a, s)].append(stem)
        w("| addr | size | models |")
        w("|:----:|:----:|--------|")
        for (a, s), ms in sorted(by_val.items()):
            w(f"| {a} | {s} | {', '.join(sorted(ms))} |")
        w()

    # ========== 6. ALIAS COLLISIONS ==========
    w("## 6. Intra-model address aliases")
    w()
    w("Same address mapped to two+ register names inside one model (e.g. "
      "`Indirect Address 1` == `Indirect Address Write`). v2 must decide a "
      "canonical name per address.")
    w()
    any_alias = False
    for stem, m in models.items():
        if m["aliases"]:
            any_alias = True
            parts = "; ".join(f"@{a}: {' == '.join(dict.fromkeys(ns))}"
                              for a, ns in sorted(m["aliases"].items()))
            w(f"- **{stem}** — {parts}")
    if not any_alias:
        w("_none_")
    w()

    # ========== 7. RARE / UNIQUE registers ==========
    w("## 7. Rare registers (present in ≤ 3 models) — model-specific extras")
    w()
    rare = [r for r in reg_rows if r["present"] <= 3]
    rare.sort(key=lambda r: (r["present"], r["name"]))
    w("| Register | Models | Which |")
    w("|----------|:------:|-------|")
    for r in rare:
        w(f"| {r['name']} | {r['present']} | {', '.join(sorted(r['per'].keys()))} |")
    w()

    # ---- machine-readable dump for downstream tooling / agents ----
    dump = {
        "n_models": n_models,
        "exact_groups": exact_groups,
        "tiers": {TIER_LABEL.get(tk, f"{tk[0]} GP@{tk[1]}"): gs
                  for tk, gs in tier_list},
        "registers": {
            r["name"]: {
                "present": r["present"],
                "addr_uniform": r["addr_uniform"],
                "size_uniform": r["size_uniform"],
                "addrs": r["addrs"], "sizes": r["sizes"],
                "per": {k: list(v) for k, v in r["per"].items()},
            } for r in reg_rows
        },
        "models": {
            s: {"proto": m["proto"], "model_number": m["model_number"],
                "n_regs": len(m["regmap"]),
                "regs": {n: list(v) for n, v in m["regmap"].items()}}
            for s, m in models.items()
        },
    }
    open(os.path.join(RON_DIR, "control_table_analysis.json"), "w").write(
        json.dumps(dump, indent=1))

    open(OUT, "w").write("\n".join(L) + "\n")

    # console summary
    print(f"models={n_models}  exact_layouts={len(exact_groups)}  "
          f"tiers={len(tier_list)}  regs={len(reg_universe)}  "
          f"universal={len(universal)}  varies={len(varies)}")
    print(f"wrote {OUT}")
    print(f"wrote {os.path.join(RON_DIR,'control_table_analysis.json')}")

if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""e-Manual page reader for the control tables (maintainer only, needs the network).

Command line, output and exit codes are the contract in
`specs/005-tables/contracts/table-tools.md` section 2 (research R-7).

    python3 control_tables/emanual.py rows URL
    python3 control_tables/emanual.py diff STEM [URL]
    python3 control_tables/emanual.py fill STEM [URL]

A page's control tables are the HTML tables whose header row names Address,
Size, Data Name and Access (matched by header name, not position). A row's
area comes from the heading right above its table, which must name EEPROM or
RAM. Rows whose address or size is not a number (the "…" rows) are skipped.

`fill` adds `access` and `area` only to a row whose address, size and name all
match one page row, and only when the row has neither field yet. It never
changes an address, size, name or any other field, keeps the row style, and
lists every row it left for a human.

Exit codes: 0 done; 1 the page broke a rule (an access other than R or RW, a
row not under an EEPROM or RAM heading) or `diff` found differences; 2 network
error, page not found, bad arguments or a table that does not parse.
"""

from __future__ import annotations

import argparse
import re
import sys
import urllib.error
import urllib.request
from html.parser import HTMLParser
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from check_tables import Ident, Record, RonError, parse_ron  # noqa: E402

EMANUAL_PREFIX = "https://emanual.robotis.com/"
USER_AGENT = "dxl_rs_v2-control-tables/1.0 (python-urllib)"
TIMEOUT = 30
TABLES_DIR = Path(__file__).resolve().parent

# Table stem -> its model's English e-Manual page (found from
# https://emanual.robotis.com/docs/en/dxl/, each opened on 2026-10-28).
PAGES: dict[str, str] = {
    "2xc430_w250": "https://emanual.robotis.com/docs/en/dxl/x/2xc430-w250/",
    "2xl430_w250": "https://emanual.robotis.com/docs/en/dxl/x/2xl430-w250/",
    "ax_12_plus": "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/",
    "ax_12a": "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/",
    "ax_12w": "https://emanual.robotis.com/docs/en/dxl/ax/ax-12w/",
    "ax_18a": "https://emanual.robotis.com/docs/en/dxl/ax/ax-18a/",
    "ax_18f": "https://emanual.robotis.com/docs/en/dxl/ax/ax-18a/",
    "h42_20_s300_ra": "https://emanual.robotis.com/docs/en/dxl/pro/h42-20-s300-ra/",
    "h54_100_s500_ra": "https://emanual.robotis.com/docs/en/dxl/pro/h54-100-s500-ra/",
    "h54_200_s500_ra": "https://emanual.robotis.com/docs/en/dxl/pro/h54-200-s500-ra/",
    "m42_10_s260_ra": "https://emanual.robotis.com/docs/en/dxl/pro/m42-10-s260-ra/",
    "m54_40_s250_ra": "https://emanual.robotis.com/docs/en/dxl/pro/m54-40-s250-ra/",
    "m54_60_s250_ra": "https://emanual.robotis.com/docs/en/dxl/pro/m54-60-s250-ra/",
    "mx_106": "https://emanual.robotis.com/docs/en/dxl/mx/mx-106-2/",
    "mx_106_p1": "https://emanual.robotis.com/docs/en/dxl/mx/mx-106/",
    "mx_28": "https://emanual.robotis.com/docs/en/dxl/mx/mx-28-2/",
    "mx_28_p1": "https://emanual.robotis.com/docs/en/dxl/mx/mx-28/",
    "mx_64": "https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/",
    "mx_64_p1": "https://emanual.robotis.com/docs/en/dxl/mx/mx-64/",
    "ph42_020_s300": "https://emanual.robotis.com/docs/en/dxl/p/ph42-020-s300-r/",
    "ph54_100_s500": "https://emanual.robotis.com/docs/en/dxl/p/ph54-100-s500-r/",
    "ph54_200_s500": "https://emanual.robotis.com/docs/en/dxl/p/ph54-200-s500-r/",
    "pm42_010_s260": "https://emanual.robotis.com/docs/en/dxl/p/pm42-010-s260-r/",
    "pm54_040_s250": "https://emanual.robotis.com/docs/en/dxl/p/pm54-040-s250-r/",
    "pm54_060_s250": "https://emanual.robotis.com/docs/en/dxl/p/pm54-060-s250-r/",
    "rh_p12_rn": "https://emanual.robotis.com/docs/en/platform/rh_p12_rna/",
    "rx_28": "https://emanual.robotis.com/docs/en/dxl/rx/rx-28/",
    "xc330_m181": "https://emanual.robotis.com/docs/en/dxl/x/xc330-m181/",
    "xc330_m181_fw52": "https://emanual.robotis.com/docs/en/dxl/x/xc330-m181/",
    "xc330_m288": "https://emanual.robotis.com/docs/en/dxl/x/xc330-m288/",
    "xc330_m288_fw52": "https://emanual.robotis.com/docs/en/dxl/x/xc330-m288/",
    "xc330_t181": "https://emanual.robotis.com/docs/en/dxl/x/xc330-t181/",
    "xc330_t181_fw52": "https://emanual.robotis.com/docs/en/dxl/x/xc330-t181/",
    "xc330_t288": "https://emanual.robotis.com/docs/en/dxl/x/xc330-t288/",
    "xc330_t288_fw52": "https://emanual.robotis.com/docs/en/dxl/x/xc330-t288/",
    "xc430_w150": "https://emanual.robotis.com/docs/en/dxl/x/xc430-w150/",
    "xc430_w240": "https://emanual.robotis.com/docs/en/dxl/x/xc430-w240/",
    "xd430_t210": "https://emanual.robotis.com/docs/en/dxl/x/xd430-t210/",
    "xd430_t350": "https://emanual.robotis.com/docs/en/dxl/x/xd430-t350/",
    "xd540_t150": "https://emanual.robotis.com/docs/en/dxl/x/xd540-t150/",
    "xd540_t270": "https://emanual.robotis.com/docs/en/dxl/x/xd540-t270/",
    "xh430_v210": "https://emanual.robotis.com/docs/en/dxl/x/xh430-v210/",
    "xh430_v350": "https://emanual.robotis.com/docs/en/dxl/x/xh430-v350/",
    "xh430_w210": "https://emanual.robotis.com/docs/en/dxl/x/xh430-w210/",
    "xh430_w350": "https://emanual.robotis.com/docs/en/dxl/x/xh430-w350/",
    "xh540_v150": "https://emanual.robotis.com/docs/en/dxl/x/xh540-v150/",
    "xh540_v270": "https://emanual.robotis.com/docs/en/dxl/x/xh540-v270/",
    "xh540_w150": "https://emanual.robotis.com/docs/en/dxl/x/xh540-w150/",
    "xh540_w270": "https://emanual.robotis.com/docs/en/dxl/x/xh540-w270/",
    "xl320": "https://emanual.robotis.com/docs/en/dxl/x/xl320/",
    "xl330_m077": "https://emanual.robotis.com/docs/en/dxl/x/xl330-m077/",
    "xl330_m077_fw52": "https://emanual.robotis.com/docs/en/dxl/x/xl330-m077/",
    "xl330_m288": "https://emanual.robotis.com/docs/en/dxl/x/xl330-m288/",
    "xl330_m288_fw52": "https://emanual.robotis.com/docs/en/dxl/x/xl330-m288/",
    "xl430_w250": "https://emanual.robotis.com/docs/en/dxl/x/xl430-w250/",
    "xm430_w210": "https://emanual.robotis.com/docs/en/dxl/x/xm430-w210/",
    "xm430_w350": "https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/",
    "xm540_w150": "https://emanual.robotis.com/docs/en/dxl/x/xm540-w150/",
    "xm540_w270": "https://emanual.robotis.com/docs/en/dxl/x/xm540-w270/",
    "xw430_t200": "https://emanual.robotis.com/docs/en/dxl/x/xw430-t200/",
    "xw430_t333": "https://emanual.robotis.com/docs/en/dxl/x/xw430-t333/",
    "xw540_h260": "https://emanual.robotis.com/docs/en/dxl/x/xw540-h260/",
    "xw540_t140": "https://emanual.robotis.com/docs/en/dxl/x/xw540-t140/",
    "xw540_t260": "https://emanual.robotis.com/docs/en/dxl/x/xw540-t260/",
    "ym070_210_a051": "https://emanual.robotis.com/docs/en/dxl/y/ym070-210-a051-rh/",
    "ym070_210_a099": "https://emanual.robotis.com/docs/en/dxl/y/ym070-210-a099-rh/",
    "ym070_210_b001": "https://emanual.robotis.com/docs/en/dxl/y/ym070-210-b001-rh/",
    "ym070_210_m001": "https://emanual.robotis.com/docs/en/dxl/y/ym070-210-m001-rh/",
    "ym070_210_r051": "https://emanual.robotis.com/docs/en/dxl/y/ym070-210-r051-rh/",
    "ym070_210_r099": "https://emanual.robotis.com/docs/en/dxl/y/ym070-210-r099-rh/",
    "ym080_230_a051": "https://emanual.robotis.com/docs/en/dxl/y/ym080-230-a051-rh/",
    "ym080_230_a099": "https://emanual.robotis.com/docs/en/dxl/y/ym080-230-a099-rh/",
    "ym080_230_b001": "https://emanual.robotis.com/docs/en/dxl/y/ym080-230-b001-rh/",
    "ym080_230_m001": "https://emanual.robotis.com/docs/en/dxl/y/ym080-230-m001-rh/",
    "ym080_230_r051": "https://emanual.robotis.com/docs/en/dxl/y/ym080-230-r051-rh/",
    "ym080_230_r099": "https://emanual.robotis.com/docs/en/dxl/y/ym080-230-r099-rh/",
}

ACCESS_VALUES = ("R", "RW")

PageRow = tuple[int, int, str, str, str]  # address, size, data name, access, area


class FetchError(Exception):
    """Exit code 2: the page could not be fetched."""


class PageError(Exception):
    """Exit code 1: the page breaks a rule (access value, row without an area)."""


def fetch(url: str) -> str:
    """Downloads `url` and returns its HTML."""
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=TIMEOUT) as response:
            charset = response.headers.get_content_charset() or "utf-8"
            return response.read().decode(charset, errors="replace")
    except urllib.error.HTTPError as err:
        raise FetchError(f"{url}: HTTP {err.code} {err.reason}") from None
    except (urllib.error.URLError, TimeoutError, OSError) as err:
        raise FetchError(f"{url}: {err}") from None


class _PageParser(HTMLParser):
    """Collects headings and tables in document order."""

    HEADINGS = ("h1", "h2", "h3", "h4", "h5", "h6")

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.events: list[tuple[str, object]] = []
        self._heading: list[str] | None = None
        self._table_depth = 0
        self._rows: list[list[str]] = []
        self._row: list[str] | None = None
        self._cell: list[str] | None = None

    def handle_starttag(self, tag: str, attrs: list) -> None:
        if tag in self.HEADINGS and self._table_depth == 0:
            self._heading = []
        elif tag == "table":
            self._table_depth += 1
            if self._table_depth == 1:
                self._rows = []
        elif self._table_depth == 1 and tag == "tr":
            self._row = []
        elif self._table_depth == 1 and tag in ("td", "th") and self._row is not None:
            self._cell = []
        elif tag == "br":
            self._text(" ")

    def handle_startendtag(self, tag: str, attrs: list) -> None:
        if tag == "br":
            self._text(" ")

    def handle_endtag(self, tag: str) -> None:
        if tag in self.HEADINGS and self._heading is not None:
            self.events.append(("heading", _clean("".join(self._heading))))
            self._heading = None
        elif tag == "table" and self._table_depth:
            self._table_depth -= 1
            if self._table_depth == 0:
                self.events.append(("table", self._rows))
        elif self._table_depth == 1 and tag in ("td", "th") and self._cell is not None:
            assert self._row is not None
            self._row.append(_clean("".join(self._cell)))
            self._cell = None
        elif self._table_depth == 1 and tag == "tr" and self._row is not None:
            self._rows.append(self._row)
            self._row = None

    def handle_data(self, data: str) -> None:
        self._text(data)

    def _text(self, data: str) -> None:
        if self._heading is not None:
            self._heading.append(data)
        if self._cell is not None:
            self._cell.append(data)


def _clean(text: str) -> str:
    return " ".join(text.split())


def _header_key(text: str) -> str:
    return re.sub(r"[^a-z]", "", text.lower())


def _columns(header: list[str]) -> dict[str, int] | None:
    """Column index of address, size, name and access, or None if this is not a control table."""
    found: dict[str, int] = {}
    for i, cell in enumerate(header):
        key = _header_key(cell)
        if key == "address":
            found.setdefault("address", i)
        elif key.startswith("size"):
            found.setdefault("size", i)
        elif key == "dataname":
            found.setdefault("name", i)
        elif key == "access":
            found.setdefault("access", i)
    return found if len(found) == 4 else None


def _area(heading: str | None) -> str | None:
    if heading is None:
        return None
    eeprom = re.search(r"\bEEPROM\b", heading) is not None
    ram = re.search(r"\bRAM\b", heading) is not None
    if eeprom and not ram:
        return "EEPROM"
    if ram and not eeprom:
        return "RAM"
    return None


def parse_page(html: str, skipped: list[str] | None = None) -> list[PageRow]:
    """Every control-table row of a page, in page order.

    Raises `PageError` on an access value other than R or RW, or on a row
    whose table is not right under an EEPROM or RAM heading. Rows whose address
    or size is not a number are appended to `skipped` (when given).
    """
    parser = _PageParser()
    parser.feed(html)
    parser.close()
    rows: list[PageRow] = []
    heading: str | None = None
    for kind, payload in parser.events:
        if kind == "heading":
            heading = payload  # type: ignore[assignment]
            continue
        table: list[list[str]] = payload  # type: ignore[assignment]
        if not table:
            continue
        cols = _columns(table[0])
        if cols is None:
            continue
        area = _area(heading)
        for cells in table[1:]:
            if len(cells) <= max(cols.values()):
                if skipped is not None:
                    skipped.append(" | ".join(cells))
                continue
            address, size = cells[cols["address"]], cells[cols["size"]]
            name, access = cells[cols["name"]], cells[cols["access"]].replace(" ", "")
            if not (address.isdigit() and size.isdigit()):
                if skipped is not None and not all(c in ("…", "...", "-", "") for c in (address, size, name)):
                    skipped.append(" | ".join(cells))
                continue
            if area is None:
                raise PageError(
                    f"row {address} {name!r}: its table is not under an EEPROM or RAM heading "
                    f"(heading above it: {heading!r})"
                )
            if access not in ACCESS_VALUES:
                raise PageError(f"row {address} {name!r}: access {access!r} is neither R nor RW")
            rows.append((int(address), int(size), name, access, area))
    return rows


def ron_string(text: str) -> str:
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def ron_row(row: PageRow) -> str:
    address, size, name, access, area = row
    return f"{{ address: {address}, size: {size}, data_name: {ron_string(name)}, access: {access}, area: {area} }},"


# --------------------------------------------------------------------------
# Table side
# --------------------------------------------------------------------------


def _table_path(stem: str) -> Path:
    return TABLES_DIR / f"{stem}.ron"


def _read_table(stem: str) -> tuple[str, list[Record]]:
    path = _table_path(stem)
    text = path.read_text(encoding="utf-8")
    table = parse_ron(text, path.name)
    rows = table.get("control_table") if isinstance(table, dict) else None
    if not isinstance(rows, list):
        raise RonError(path.name, 1, 1, "missing `control_table`")
    return text, [r for r in rows if isinstance(r, Record)]


def _key(row: Record) -> tuple[int, int, str]:
    return row.get("address"), row.get("size"), row.get("data_name")  # type: ignore[return-value]


def _fields(row: Record) -> str:
    access = row.get("access")
    area = row.get("area")
    return f"{access if access is not None else '-'} {area if area is not None else '-'}"


def compare(table_rows: list[Record], page_rows: list[PageRow]) -> dict[str, list[str]]:
    """The four difference categories of the contract, as printable lines."""
    by_key: dict[tuple[int, int, str], set[tuple[str, str]]] = {}
    by_slot: dict[tuple[int, int], list[str]] = {}
    for address, size, name, access, area in page_rows:
        by_key.setdefault((address, size, name), set()).add((access, area))
        names = by_slot.setdefault((address, size), [])
        if name not in names:
            names.append(name)
    table_keys = {_key(r) for r in table_rows}
    table_slots = {(r.get("address"), r.get("size")) for r in table_rows}
    result: dict[str, list[str]] = {"only in table": [], "only on page": [], "different name": [], "different access/area": []}
    for row in table_rows:
        address, size, name = _key(row)
        if (address, size, name) in by_key:
            fields = by_key[(address, size, name)]
            access, area = row.get("access"), row.get("area")
            if access is None and area is None:
                continue
            ok = any(
                (access is None or access == Ident(a)) and (area is None or area == Ident(r)) for a, r in fields
            )
            if not ok:
                page = ", ".join(f"{a} {r}" for a, r in sorted(fields))
                result["different access/area"].append(
                    f'{address} {size} {ron_string(name)}: table {_fields(row)}, page {page}'
                )
        elif (address, size) in by_slot:
            page_names = ", ".join(ron_string(n) for n in by_slot[(address, size)])
            note = ""
            if any(n.lower() == name.lower() for n in by_slot[(address, size)]):
                note = " (case only)"
            result["different name"].append(f"{address} {size} table {ron_string(name)}, page {page_names}{note}")
        else:
            result["only in table"].append(f"{address} {size} {ron_string(name)}")
    seen: set[tuple[int, int, str]] = set()
    for address, size, name, access, area in page_rows:
        key = (address, size, name)
        if key in seen:
            continue
        seen.add(key)
        if key not in table_keys and (address, size) not in table_slots:
            result["only on page"].append(f"{address} {size} {ron_string(name)} {access} {area}")
    return result


def fill_text(text: str, rows: list[Record], page_rows: list[PageRow]) -> tuple[str, list[str], list[str], int]:
    """Returns (new text, filled lines, left lines, rows already set)."""
    by_key: dict[tuple[int, int, str], set[tuple[str, str]]] = {}
    by_slot: dict[tuple[int, int], list[str]] = {}
    for address, size, name, access, area in page_rows:
        by_key.setdefault((address, size, name), set()).add((access, area))
        by_slot.setdefault((address, size), []).append(name)
    edits: list[tuple[int, str]] = []
    filled: list[str] = []
    left: list[str] = []
    already = 0
    for row in rows:
        address, size, name = _key(row)
        label = f"{address} {size} {ron_string(str(name))}"
        has_access, has_area = "access" in row, "area" in row
        if has_access and has_area:
            already += 1
            continue
        if has_access or has_area:
            left.append(f"{label}: has only one of `access` / `area`")
            continue
        if list(row) != ["address", "size", "data_name"]:
            left.append(f"{label}: fields are not exactly `address, size, data_name`")
            continue
        fields = by_key.get((address, size, name))
        if not fields:
            others = [n for n in by_slot.get((address, size), []) if n != name]
            if others:
                left.append(
                    f"{label}: no page row with this name; page has "
                    + ", ".join(ron_string(n) for n in dict.fromkeys(others))
                    + " at this address and size"
                )
            else:
                left.append(f"{label}: no page row at this address and size")
            continue
        if len(fields) > 1:
            left.append(f"{label}: the page lists it with different access/area: {sorted(fields)}")
            continue
        access, area = next(iter(fields))
        edits.extend(_row_edits(text, row, access, area))
        filled.append(f"{label} -> {access} {area}")
    for pos, insert in sorted(edits, key=lambda e: e[0], reverse=True):
        text = text[:pos] + insert + text[pos:]
    return text, filled, left, already


def _row_edits(text: str, row: Record, access: str, area: str) -> list[tuple[int, str]]:
    value_end = row.spans["data_name"][1]
    if "\n" not in text[row.start : row.end]:
        return [(value_end, f", access: {access}, area: {area}")]
    line_end = text.find("\n", value_end)
    line_start = text.rfind("\n", 0, value_end) + 1
    indent = re.match(r"[ \t]*", text[line_start:]).group()  # type: ignore[union-attr]
    edits = []
    after = text[value_end:line_end]
    if not after.lstrip().startswith(","):
        edits.append((value_end, ","))
    edits.append((line_end, f"\n{indent}access: {access},\n{indent}area: {area},"))
    return edits


def _strip_filled(value: object) -> object:
    """The parsed table with `access`/`area` removed from every row, for the before/after check."""
    if isinstance(value, dict):
        return {k: _strip_filled(v) for k, v in value.items() if k not in ("access", "area")}
    if isinstance(value, list):
        return [_strip_filled(v) for v in value]
    return value


# --------------------------------------------------------------------------
# Commands
# --------------------------------------------------------------------------


def _page_rows(url: str) -> list[PageRow]:
    skipped: list[str] = []
    rows = parse_page(fetch(url), skipped)
    for line in skipped:
        print(f"note: skipped page row: {line}", file=sys.stderr)
    if not rows:
        raise PageError(f"{url}: no control-table rows found")
    return rows


def cmd_rows(url: str) -> int:
    rows = _page_rows(url)
    print(f"// {len(rows)} rows from {url}")
    for row in rows:
        print(ron_row(row))
    return 0


def cmd_diff(stem: str, url: str) -> int:
    page_rows = _page_rows(url)
    _text, rows = _read_table(stem)
    result = compare(rows, page_rows)
    titles = {
        "only in table": "only in table",
        "only on page": "only on page",
        "different name": "different name at the same address and size",
        "different access/area": "different access/area",
    }
    total = 0
    print(f"{stem}.ron vs {url}")
    for key, title in titles.items():
        lines = result[key]
        total += len(lines)
        print(f"{title} ({len(lines)}):")
        for line in lines:
            print(f"  {line}")
    print(f"{total} differences")
    return 1 if total else 0


def cmd_fill(stem: str, url: str) -> int:
    page_rows = _page_rows(url)
    text, rows = _read_table(stem)
    path = _table_path(stem)
    new_text, filled, left, already = fill_text(text, rows, page_rows)
    if filled:
        before = parse_ron(text, path.name)
        after = parse_ron(new_text, path.name)
        if _strip_filled(before) != _strip_filled(after):
            raise PageError(f"{path.name}: fill would change more than access/area; nothing written")
        path.write_text(new_text, encoding="utf-8")
    print(f"{stem}.ron from {url}")
    print(f"filled {len(filled)} rows, {already} already had access and area, left {len(left)} rows for a human")
    for line in filled:
        print(f"  filled {line}")
    for line in left:
        print(f"  left {line}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="emanual.py",
        description="Read a model's control table from the ROBOTIS e-Manual.",
    )
    sub = parser.add_subparsers(dest="command", required=True)
    rows = sub.add_parser("rows", help="print the page's control-table rows")
    rows.add_argument("url")
    for name, text in (
        ("diff", "compare a table with its page"),
        ("fill", "add access and area to rows that match the page"),
    ):
        cmd = sub.add_parser(name, help=text)
        cmd.add_argument("stem")
        cmd.add_argument("url", nargs="?")
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    try:
        args = parser.parse_args(argv)
    except SystemExit as exc:
        return int(exc.code or 0)
    url = args.url
    if args.command != "rows":
        if not _table_path(args.stem).is_file():
            print(f"error: no table {args.stem}.ron in {TABLES_DIR}", file=sys.stderr)
            return 2
        url = url or PAGES.get(args.stem)
        if url is None:
            print(f"error: no URL given and {args.stem!r} is not in PAGES", file=sys.stderr)
            return 2
    if not url.startswith(EMANUAL_PREFIX):
        print(f"error: not an e-Manual URL (must start with {EMANUAL_PREFIX}): {url}", file=sys.stderr)
        return 2
    try:
        if args.command == "rows":
            return cmd_rows(url)
        if args.command == "diff":
            return cmd_diff(args.stem, url)
        return cmd_fill(args.stem, url)
    except (FetchError, RonError) as err:
        print(f"error: {err}", file=sys.stderr)
        return 2
    except PageError as err:
        print(f"error: {err}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())

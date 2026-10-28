#!/usr/bin/env python3
"""Table check for `control_tables/`: validates every table and `dynamixel.ron`.

Command line, output and exit codes are the contract in
`specs/005-tables/contracts/table-tools.md` section 1; the rules are those of
`specs/005-tables/contracts/ron-schema.md` sections 1 and 2.

    python3 control_tables/check_tables.py [--dir DIR] [--today YYYY-MM-DD] [--markdown] [STEM ...]

Exit codes: 0 no error, 1 a rule is broken, 2 bad arguments, a file that does
not parse or a missing `dynamixel.ron`. No network.
"""

from __future__ import annotations

import argparse
import datetime
import re
import sys
from pathlib import Path

DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
MAP_FILE = "dynamixel.ron"
EMANUAL_PREFIX = "https://emanual.robotis.com/"


# --------------------------------------------------------------------------
# RON subset parser (research R-6)
# --------------------------------------------------------------------------


class RonError(Exception):
    """A file that does not parse: names the file, line and column."""

    def __init__(self, file: str, line: int, col: int, message: str) -> None:
        super().__init__(f"{file}:{line}:{col}: {message}")
        self.file = file
        self.line = line
        self.col = col
        self.message = message


class Ident:
    """A bare identifier such as `Unverified`, `P1`, `RW` or `EEPROM`."""

    __slots__ = ("name",)

    def __init__(self, name: str) -> None:
        self.name = name

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Ident) and other.name == self.name

    def __hash__(self) -> int:
        return hash(("Ident", self.name))

    def __repr__(self) -> str:
        return f"Ident({self.name!r})"

    def __str__(self) -> str:
        return self.name


class Tagged:
    """A tagged value: `Verified(url: …, verified: …)` (args a dict) or `SameNumberAs(12)` (args a list)."""

    __slots__ = ("name", "args")

    def __init__(self, name: str, args: dict | list) -> None:
        self.name = name
        self.args = args

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Tagged) and other.name == self.name and other.args == self.args

    def __hash__(self) -> int:
        return hash(("Tagged", self.name))

    def __repr__(self) -> str:
        return f"Tagged({self.name!r}, {self.args!r})"

    def __str__(self) -> str:
        return self.name


class Record(dict):
    """A `( key: value )` or `{ key: value }` record.

    A plain dict that also remembers where it sits in the source text, so
    `emanual.py fill` can edit a row in place: `start` and `end` are the
    offsets of the opening and closing bracket, `line` the line of the
    opening bracket, and `spans[key]` the (start, end) offsets of each value.
    """

    def __init__(self) -> None:
        super().__init__()
        self.start = 0
        self.end = 0
        self.line = 0
        self.spans: dict[str, tuple[int, int]] = {}


_TOKEN_RE = re.compile(
    r"""
    (?P<ws>\s+)
  | (?P<comment>//[^\n]*)
  | (?P<block>/\*.*?\*/)
  | (?P<float>-?(?:\d+\.\d*(?:[eE][+-]?\d+)?|\d+[eE][+-]?\d+|\.\d+))
  | (?P<int>-?\d+)
  | (?P<ident>[A-Za-z_][A-Za-z0-9_]*)
  | (?P<punct>[()\[\]{}:,])
  | (?P<string>")
    """,
    re.VERBOSE | re.DOTALL,
)

_ESCAPES = {'"': '"', "\\": "\\", "/": "/", "n": "\n", "t": "\t", "r": "\r", "0": "\0", "'": "'"}


class _Token:
    __slots__ = ("kind", "value", "start", "end", "line", "col")

    def __init__(self, kind: str, value: object, start: int, end: int, line: int, col: int) -> None:
        self.kind = kind
        self.value = value
        self.start = start
        self.end = end
        self.line = line
        self.col = col


def _tokenize(text: str, file: str) -> list[_Token]:
    tokens: list[_Token] = []
    pos = 0
    line = 1
    line_start = 0
    length = len(text)
    while pos < length:
        col = pos - line_start + 1
        m = _TOKEN_RE.match(text, pos)
        if m is None:
            raise RonError(file, line, col, f"unexpected character {text[pos]!r}")
        kind = m.lastgroup
        if kind == "string":
            value, end = _read_string(text, pos, file, line, col)
            tokens.append(_Token("str", value, pos, end, line, col))
        else:
            end = m.end()
            raw = m.group()
            if kind == "float":
                tokens.append(_Token("float", float(raw), pos, end, line, col))
            elif kind == "int":
                tokens.append(_Token("int", int(raw), pos, end, line, col))
            elif kind == "ident":
                tokens.append(_Token("ident", raw, pos, end, line, col))
            elif kind == "punct":
                tokens.append(_Token("punct", raw, pos, end, line, col))
        newlines = text.count("\n", pos, end)
        if newlines:
            line += newlines
            line_start = text.rfind("\n", pos, end) + 1
        pos = end
    tokens.append(_Token("eof", None, length, length, line, length - line_start + 1))
    return tokens


def _read_string(text: str, start: int, file: str, line: int, col: int) -> tuple[str, int]:
    out: list[str] = []
    pos = start + 1
    while pos < len(text):
        c = text[pos]
        if c == '"':
            return "".join(out), pos + 1
        if c == "\n":
            break
        if c == "\\":
            pos += 1
            if pos >= len(text):
                break
            e = text[pos]
            if e in _ESCAPES:
                out.append(_ESCAPES[e])
            elif e == "u" and text.startswith("{", pos + 1):
                close = text.find("}", pos)
                if close < 0:
                    break
                out.append(chr(int(text[pos + 2 : close], 16)))
                pos = close
            else:
                raise RonError(file, line, col, f"unknown escape \\{e} in string")
        else:
            out.append(c)
        pos += 1
    raise RonError(file, line, col, "unterminated string")


class _Parser:
    def __init__(self, text: str, file: str) -> None:
        self.file = file
        self.tokens = _tokenize(text, file)
        self.i = 0

    def peek(self, ahead: int = 0) -> _Token:
        return self.tokens[min(self.i + ahead, len(self.tokens) - 1)]

    def next(self) -> _Token:
        tok = self.tokens[self.i]
        self.i += 1
        return tok

    def error(self, tok: _Token, message: str) -> RonError:
        return RonError(self.file, tok.line, tok.col, message)

    def is_punct(self, tok: _Token, value: str) -> bool:
        return tok.kind == "punct" and tok.value == value

    def expect(self, value: str) -> _Token:
        tok = self.next()
        if not self.is_punct(tok, value):
            raise self.error(tok, f"expected `{value}`, found {self.describe(tok)}")
        return tok

    @staticmethod
    def describe(tok: _Token) -> str:
        if tok.kind == "eof":
            return "end of file"
        if tok.kind == "str":
            return f"string {tok.value!r}"
        return f"`{tok.value}`"

    def parse(self) -> object:
        value = self.value()
        tok = self.peek()
        if tok.kind != "eof":
            raise self.error(tok, f"unexpected {self.describe(tok)} after the value")
        return value

    def value(self) -> object:
        tok = self.peek()
        if tok.kind in ("str", "int", "float"):
            self.next()
            return tok.value
        if tok.kind == "ident":
            self.next()
            if self.is_punct(self.peek(), "("):
                return Tagged(tok.value, self.paren())
            return Ident(tok.value)
        if tok.kind == "punct":
            if tok.value == "(":
                return self.paren()
            if tok.value == "{":
                return self.brace()
            if tok.value == "[":
                return self.list()
        raise self.error(tok, f"expected a value, found {self.describe(tok)}")

    def starts_record(self) -> bool:
        return self.peek().kind == "ident" and self.is_punct(self.peek(1), ":")

    def paren(self) -> object:
        open_tok = self.expect("(")
        if self.is_punct(self.peek(), ")"):
            return self.record_tail(open_tok, ")")
        if self.starts_record():
            return self.record_tail(open_tok, ")")
        items = self.items(")")
        return items

    def brace(self) -> object:
        open_tok = self.expect("{")
        if self.is_punct(self.peek(), "}") or self.starts_record():
            return self.record_tail(open_tok, "}")
        result: dict = {}
        while not self.is_punct(self.peek(), "}"):
            key_tok = self.peek()
            if key_tok.kind not in ("int", "str"):
                raise self.error(key_tok, f"expected a map key, found {self.describe(key_tok)}")
            self.next()
            if key_tok.value in result:
                raise self.error(key_tok, f"duplicate key {key_tok.value}")
            self.expect(":")
            result[key_tok.value] = self.value()
            if not self.comma_or_close("}"):
                break
        self.expect("}")
        return result

    def record_tail(self, open_tok: _Token, close: str) -> Record:
        record = Record()
        record.start = open_tok.start
        record.line = open_tok.line
        while not self.is_punct(self.peek(), close):
            key_tok = self.next()
            if key_tok.kind != "ident":
                raise self.error(key_tok, f"expected a field name, found {self.describe(key_tok)}")
            if key_tok.value in record:
                raise self.error(key_tok, f"duplicate key `{key_tok.value}`")
            self.expect(":")
            value_start = self.peek().start
            record[key_tok.value] = self.value()
            record.spans[key_tok.value] = (value_start, self.tokens[self.i - 1].end)
            if not self.comma_or_close(close):
                break
        record.end = self.expect(close).start
        return record

    def list(self) -> list:
        self.expect("[")
        return self.items("]")

    def items(self, close: str) -> list:
        items: list = []
        while not self.is_punct(self.peek(), close):
            items.append(self.value())
            if not self.comma_or_close(close):
                break
        self.expect(close)
        return items

    def comma_or_close(self, close: str) -> bool:
        """Consumes a separating comma; returns False when the next token closes the group."""
        tok = self.peek()
        if self.is_punct(tok, ","):
            self.next()
            return True
        if self.is_punct(tok, close):
            return False
        raise self.error(tok, f"expected `,` or `{close}`, found {self.describe(tok)}")


def parse_ron(text: str, file: str = "<string>") -> object:
    """Parses the RON subset the tables use into plain Python values.

    Records (`( … )` with named fields, and every `{ key: … }`) become dicts
    (`Record`), `{ INT: … }` maps dicts with integer keys, lists and tuples
    lists, bare identifiers `Ident`, `Name(…)` values `Tagged`. A duplicate key
    in any record or map raises `RonError`.
    """
    return _Parser(text, file).parse()


# --------------------------------------------------------------------------
# Loading
# --------------------------------------------------------------------------


class LoadError(Exception):
    """Exit code 2: a missing `dynamixel.ron` or a file that does not parse."""


def load(directory: Path) -> tuple[object, dict[str, object]]:
    """Reads `dynamixel.ron` and every other `*.ron` in `directory` (stem -> value)."""
    map_path = directory / MAP_FILE
    if not map_path.is_file():
        raise LoadError(f"{map_path}: missing {MAP_FILE}")
    tables: dict[str, object] = {}
    for path in sorted(directory.glob("*.ron")):
        if path.name == MAP_FILE:
            continue
        tables[path.stem] = parse_ron(path.read_text(encoding="utf-8"), path.name)
    model_map = parse_ron(map_path.read_text(encoding="utf-8"), MAP_FILE)
    return model_map, tables


# --------------------------------------------------------------------------
# Checking
# --------------------------------------------------------------------------


class Report:
    """Errors of one run, each `(file, location, message)`."""

    def __init__(self) -> None:
        self.errors: list[tuple[str, str, str]] = []

    def add(self, file: str, location: str, message: str) -> None:
        self.errors.append((file, location, message))

    def lines(self) -> list[str]:
        return [f"{f}: {loc}: {msg}" if loc else f"{f}: {msg}" for f, loc, msg in self.errors]


def _is_record(value: object) -> bool:
    return isinstance(value, dict) and all(isinstance(k, str) for k in value)


def _row_location(row: dict) -> str:
    address = row.get("address", "?")
    name = row.get("data_name", "?")
    return f'row {address} "{name}"'


def table_rows(table: object) -> list[dict]:
    """The well-formed rows of a table (records with an int address and size and a string name)."""
    if not _is_record(table):
        return []
    rows = table.get("control_table")
    if not isinstance(rows, list):
        return []
    return [
        r
        for r in rows
        if _is_record(r)
        and isinstance(r.get("address"), int)
        and isinstance(r.get("size"), int)
        and isinstance(r.get("data_name"), str)
    ]


def check_table_shape(stem: str, table: object, report: Report) -> None:
    """`control_table` is a non-empty list of rows with `address`, `size` and `data_name`."""
    file = f"{stem}.ron"
    if not _is_record(table):
        report.add(file, "", "the file is not a record `( … )`")
        return
    rows = table.get("control_table")
    if rows is None:
        report.add(file, "", "missing `control_table`")
        return
    if not isinstance(rows, list) or not rows:
        report.add(file, "`control_table`", "must be a non-empty list of rows")
        return
    for i, row in enumerate(rows):
        if not _is_record(row):
            report.add(file, f"`control_table` item {i}", "not a row record")
            continue
        loc = _row_location(row)
        address, size, name = row.get("address"), row.get("size"), row.get("data_name")
        if not isinstance(address, int) or not 0 <= address <= 0xFFFF:
            report.add(file, loc, "`address` must be an integer 0–65535")
        if not isinstance(size, int) or not 1 <= size <= 0xFF:
            report.add(file, loc, "`size` must be an integer 1–255")
        if not isinstance(name, str) or not name:
            report.add(file, loc, "`data_name` must be a non-empty string")


def check_url(file: str, location: str, url: object, report: Report) -> None:
    """An e-Manual URL: a string starting with `https://emanual.robotis.com/`."""
    if not isinstance(url, str):
        report.add(file, location, "must be a URL string")
    elif not url.startswith(EMANUAL_PREFIX):
        report.add(file, location, f"URL {url!r} is not on {EMANUAL_PREFIX}")


def check_source(stem: str, table: dict, today: datetime.date, report: Report) -> None:
    """`source: Unverified` or `source: Verified(url, verified)` (FR-001, FR-002)."""
    file = f"{stem}.ron"
    if "source" not in table:
        report.add(file, "", "missing `source` (Unverified or Verified(url, verified))")
        return
    source = table["source"]
    if source == Ident("Unverified"):
        return
    if not (isinstance(source, Tagged) and source.name == "Verified" and _is_record(source.args)):
        report.add(file, "`source`", f"must be `Unverified` or `Verified(url: …, verified: …)`, found {source!r}")
        return
    args = source.args
    extra = sorted(set(args) - {"url", "verified"})
    if extra:
        report.add(file, "`source`", f"unknown field(s) {', '.join(extra)}")
    if "url" not in args:
        report.add(file, "`source`", "missing `url`")
    else:
        check_url(file, "`source` url", args["url"], report)
    date = args.get("verified")
    if date is None:
        report.add(file, "`source`", "missing `verified` date")
    elif not isinstance(date, str) or not DATE_RE.match(date):
        report.add(file, "`source` verified", f"{date!r} is not an ISO date YYYY-MM-DD")
    else:
        try:
            day = datetime.date.fromisoformat(date)
        except ValueError:
            report.add(file, "`source` verified", f"{date!r} is not a real date")
        else:
            if day > today:
                report.add(file, "`source` verified", f"{date} is in the future (today is {today.isoformat()})")


INSTRUCTIONS = (
    "Ping", "Read", "Write", "RegWrite", "Action", "FactoryReset", "Reboot", "Clear",
    "ControlTableBackup", "SyncRead", "SyncWrite", "FastSyncRead", "BulkRead", "BulkWrite",
    "FastBulkRead",
)  # fmt: skip
P1_INSTRUCTIONS = ("Ping", "Read", "Write", "RegWrite", "Action", "FactoryReset", "Reboot", "SyncWrite", "BulkRead")
PROTOCOLS = ("P1", "P2")


def _protocol_names(table: dict) -> list[str] | None:
    """The table's protocols as names, or None when `protocols` is not a list of identifiers."""
    protocols = table.get("protocols")
    if not isinstance(protocols, list) or not all(isinstance(p, Ident) for p in protocols):
        return None
    return [p.name for p in protocols]


def check_instructions(stem: str, table: dict, report: Report) -> None:
    """`instructions` on verified tables only, one entry or more per protocol (data-model "Instruction entry")."""
    file = f"{stem}.ron"
    is_verified = isinstance(table.get("source"), Tagged)
    entries = table.get("instructions")
    if entries is None:
        if is_verified:
            report.add(file, "`instructions`", "missing; a verified table lists its instructions per protocol")
        return
    if not is_verified:
        report.add(file, "`instructions`", "allowed on verified tables only")
        return
    if not isinstance(entries, list) or not entries:
        report.add(file, "`instructions`", "must be a non-empty list of entries")
        return
    protocols = _protocol_names(table)
    seen: dict[str, list[object]] = {}
    for i, entry in enumerate(entries):
        loc = f"`instructions` entry {i}"
        if not _is_record(entry):
            report.add(file, loc, "must be a record `(protocol: …, source: …, list: […])`")
            continue
        unknown = sorted(set(entry) - {"protocol", "source", "list", "from_fw"})
        if unknown:
            report.add(file, loc, f"unknown field(s) {', '.join(unknown)}")
        protocol = entry.get("protocol")
        name = protocol.name if isinstance(protocol, Ident) else None
        if name not in PROTOCOLS:
            report.add(file, loc, f"`protocol` must be P1 or P2, found {protocol!r}")
        elif protocols is not None and name not in protocols:
            report.add(file, loc, f"`protocol` {name} is not in the table's `protocols`")
        if "source" not in entry:
            report.add(file, loc, "missing `source` (the protocol page URL)")
        else:
            check_url(file, f"{loc} `source`", entry["source"], report)
        names = entry.get("list")
        if not isinstance(names, list) or not names:
            report.add(file, loc, "`list` must be a non-empty list of instruction names")
        else:
            listed: list[str] = []
            for item in names:
                if not isinstance(item, Ident) or item.name not in INSTRUCTIONS:
                    report.add(file, loc, f"unknown instruction {item!r}")
                    continue
                if item.name in listed:
                    report.add(file, loc, f"{item.name} listed twice")
                listed.append(item.name)
                if name == "P1" and item.name not in P1_INSTRUCTIONS:
                    report.add(file, loc, f"{item.name} is not a Protocol 1.0 instruction")
        from_fw = entry.get("from_fw")
        if from_fw is not None and (not isinstance(from_fw, int) or not 0 <= from_fw <= 255):
            report.add(file, loc, f"`from_fw` must be an integer 0–255, found {from_fw!r}")
        if name in PROTOCOLS:
            if from_fw in seen.setdefault(name, []):
                shown = "none" if from_fw is None else from_fw
                report.add(file, loc, f"two {name} entries with the same `from_fw` ({shown})")
            seen[name].append(from_fw)
    for protocol in protocols or []:
        if protocol in PROTOCOLS and protocol not in seen:
            report.add(file, "`instructions`", f"no entry for {protocol}, which is in `protocols`")


ACCESS_VALUES = ("R", "RW")
AREA_VALUES = ("EEPROM", "RAM")
MODE_NAMES = ("Operating Mode", "Control Mode")
MODE_LISTS = ("bounded", "unbounded", "no_position")
# Role rows (contract 4.3): name -> (access, area); None means any.
ROLES = {
    "Model Number": ("R", "EEPROM"),
    "Firmware Version": (None, None),
    "Torque Enable": ("RW", "RAM"),
    "Goal Position": ("RW", "RAM"),
    "Present Position": ("R", "RAM"),
}
LIMIT_FORMS = (("Min Position Limit", "Max Position Limit"), ("CW Angle Limit", "CCW Angle Limit"))
TYPE_INFO_KEYS = (
    "value_of_zero_radian_position",
    "value_of_min_radian_position",
    "value_of_max_radian_position",
    "min_radian",
    "max_radian",
)


def const_ident(data_name: str) -> str:
    """`Secondary(Shadow) ID` -> `SECONDARY_SHADOW_ID`, as xtask names register constants."""
    return "_".join(part.upper() for part in re.split(r"[^A-Za-z0-9]+", data_name) if part)


def check_protocols(stem: str, table: dict, report: Report) -> None:
    """`protocols`: a non-empty list of P1 / P2 without duplicates (FR-005)."""
    file = f"{stem}.ron"
    protocols = table.get("protocols")
    if protocols is None:
        report.add(file, "", "missing `protocols` ([P1], [P2] or [P1, P2])")
        return
    if not isinstance(protocols, list):
        report.add(file, "`protocols`", f"must be a list such as [P1, P2], found {protocols!r}")
        return
    if not protocols:
        report.add(file, "`protocols`", "must not be empty")
        return
    seen: list[str] = []
    for item in protocols:
        name = item.name if isinstance(item, Ident) else None
        if name not in PROTOCOLS:
            report.add(file, "`protocols`", f"unknown protocol {item!r} (P1 or P2)")
        elif name in seen:
            report.add(file, "`protocols`", f"{name} listed twice")
        else:
            seen.append(name)


def check_row_fields(stem: str, table: dict, report: Report) -> None:
    """Every row has `access` (R or RW) and `area` (EEPROM or RAM) (FR-007)."""
    file = f"{stem}.ron"
    for row in table_rows(table):
        loc = _row_location(row)
        for key, allowed in (("access", ACCESS_VALUES), ("area", AREA_VALUES)):
            if key not in row:
                report.add(file, loc, f"missing `{key}`")
            elif not (isinstance(row[key], Ident) and row[key].name in allowed):
                report.add(file, loc, f"`{key}` must be {' or '.join(allowed)}, found {row[key]}")
        unknown = sorted(set(row) - {"address", "size", "data_name", "access", "area"})
        if unknown:
            report.add(file, loc, f"unknown field(s) {', '.join(unknown)}")


def _has(row: dict, access: str | None, area: str | None) -> bool:
    return (access is None or row.get("access") == Ident(access)) and (area is None or row.get("area") == Ident(area))


def mode_rows(table: object) -> list[dict]:
    return [r for r in table_rows(table) if r["data_name"] in MODE_NAMES]


def check_mode_source(stem: str, table: dict, report: Report) -> None:
    """Exactly one mode source, and `operating_modes` exactly with a mode row (FR-020, research R-15)."""
    file = f"{stem}.ron"
    rows = table_rows(table)
    names = [r["data_name"] for r in rows]
    op, cm = names.count("Operating Mode"), names.count("Control Mode")
    pair = "CW Angle Limit" in names and "CCW Angle Limit" in names
    any_cw = "CW Angle Limit" in names or "CCW Angle Limit" in names
    if op > 1:
        report.add(file, "", f"{op} `Operating Mode` rows; a table has at most one")
    if cm > 1:
        report.add(file, "", f"{cm} `Control Mode` rows; a table has at most one")
    if op and cm:
        report.add(file, "", "both an `Operating Mode` and a `Control Mode` row (two mode sources)")
    if op and any_cw:
        report.add(file, "", "an `Operating Mode` row beside CW/CCW Angle Limit rows (two mode sources)")
    if cm and stem != "xl320":
        report.add(file, "", "`Control Mode` is accepted only in xl320.ron (contract 4.3); raise a second one first")
    if not op and not cm and not pair:
        report.add(
            file, "", "no mode source: no `Operating Mode` or `Control Mode` row and no CW/CCW Angle Limit pair"
        )
    for row in mode_rows(table):
        if row["size"] != 1 or not _has(row, "RW", "EEPROM"):
            report.add(
                file,
                _row_location(row),
                f"the mode register must be size 1, RW, EEPROM (found size {row['size']}, "
                f"{row.get('access', '-')}, {row.get('area', '-')})",
            )
    modes = table.get("operating_modes")
    has_mode_row = bool(op or cm)
    if modes is None:
        if has_mode_row:
            report.add(file, "`operating_modes`", "missing; a table with a mode row lists what each mode value means")
        return
    if not has_mode_row:
        report.add(file, "`operating_modes`", "present without an `Operating Mode` or `Control Mode` row")
    check_operating_modes(file, modes, report)


def check_operating_modes(file: str, modes: object, report: Report) -> None:
    loc = "`operating_modes`"
    if not _is_record(modes):
        report.add(file, loc, "must be `(bounded: [...], unbounded: [...], no_position: [...])`")
        return
    for key in MODE_LISTS:
        if key not in modes:
            report.add(file, loc, f"missing `{key}`")
    unknown = sorted(set(modes) - set(MODE_LISTS))
    if unknown:
        report.add(file, loc, f"unknown key(s) {', '.join(unknown)}")
    where: dict[int, list[str]] = {}
    for key in MODE_LISTS:
        values = modes.get(key)
        if values is None:
            continue
        if not isinstance(values, list):
            report.add(file, loc, f"`{key}` must be a list of mode values, found {values!r}")
            continue
        for value in values:
            if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 255:
                report.add(file, loc, f"`{key}`: {value!r} is not a mode value 0–255")
                continue
            if key not in where.setdefault(value, []):
                where[value].append(key)
    for value, keys in sorted(where.items()):
        if len(keys) > 1:
            both = " and ".join(f"`{k}`" for k in keys)
            report.add(file, loc, f"value {value} in both {both}")


def check_roles(stem: str, table: dict, info: MapInfo, report: Report) -> None:
    """Role rules for a mapped table (contract 4.3, xtask checks 10–12, 14–16)."""
    file = f"{stem}.ron"
    rows = table_rows(table)
    by_name: dict[str, list[dict]] = {}
    for row in rows:
        by_name.setdefault(row["data_name"], []).append(row)
    for name, (access, area) in ROLES.items():
        if name not in by_name:
            report.add(file, "", f"missing role row `{name}`")
            continue
        for row in by_name[name]:
            if not _has(row, access, area):
                report.add(
                    file,
                    _row_location(row),
                    f"role `{name}` must be {access}/{area}, found {row.get('access', '-')}/{row.get('area', '-')}",
                )
    for row in by_name.get("Model Number", []):
        if (row["address"], row["size"]) != (0, 2):
            report.add(file, _row_location(row), "`Model Number` must be at address 0, size 2")
    complete = [form for form in LIMIT_FORMS if all(n in by_name for n in form)]
    for form in LIMIT_FORMS:
        present = [n for n in form if n in by_name]
        if len(present) == 1:
            missing = next(n for n in form if n not in by_name)
            report.add(file, "", f"half a limit pair: `{present[0]}` without `{missing}`")
    if len(complete) == 2:
        report.add(file, "", "both limit forms (Min/Max Position Limit and CW/CCW Angle Limit); exactly one is allowed")
    elif not complete:
        report.add(file, "", "no complete limit pair (Min/Max Position Limit or CW/CCW Angle Limit)")
    for form in complete:
        for name in form:
            for row in by_name[name]:
                if not _has(row, "RW", "EEPROM"):
                    report.add(
                        file,
                        _row_location(row),
                        f"limit `{name}` must be RW/EEPROM, found {row.get('access', '-')}/{row.get('area', '-')}",
                    )
    if info.under(stem, "p1"):
        role_names = set(ROLES) | set(MODE_NAMES) | {n for form in LIMIT_FORMS for n in form}
        for row in rows:
            if row["data_name"] in role_names and row["address"] > 255:
                report.add(
                    file,
                    _row_location(row),
                    "role or mode row above address 255 in a table mapped under `p1` (Protocol 1 addresses are one byte)",
                )
    constants: dict[str, dict] = {}
    for row in rows:
        ident = const_ident(row["data_name"])
        if ident in constants:
            first = constants[ident]
            report.add(
                file,
                _row_location(row),
                f"gives the register constant `{ident}`, like {_row_location(first)}",
            )
        else:
            constants[ident] = row
    check_type_info(file, table.get("type_info"), report)


def check_type_info(file: str, scale: object, report: Report) -> None:
    """Scale sanity (xtask check 15): max > min for values and radians, zero within min..=max."""
    loc = "`type_info`"
    if scale is None:
        report.add(file, loc, "missing (the position scale)")
        return
    if not _is_record(scale):
        report.add(file, loc, "must be a record")
        return
    missing = [k for k in TYPE_INFO_KEYS if k not in scale]
    if missing:
        report.add(file, loc, f"missing {', '.join(missing)}")
        return
    zero, lo, hi = (scale[k] for k in TYPE_INFO_KEYS[:3])
    rmin, rmax = scale["min_radian"], scale["max_radian"]
    if not all(isinstance(v, int) and not isinstance(v, bool) for v in (zero, lo, hi)):
        report.add(file, loc, "the three position values must be integers")
        return
    if not all(isinstance(v, (int, float)) and not isinstance(v, bool) for v in (rmin, rmax)):
        report.add(file, loc, "`min_radian` and `max_radian` must be numbers")
        return
    if hi <= lo:
        report.add(file, loc, f"value_of_max_radian_position ({hi}) must be above value_of_min_radian_position ({lo})")
    if rmax <= rmin:
        report.add(file, loc, f"max_radian ({rmax}) must be above min_radian ({rmin})")
    if not lo <= zero <= hi:
        report.add(file, loc, f"value_of_zero_radian_position ({zero}) must lie within {lo}..={hi}")


def flag_text(table: object) -> str:
    """The flag column: `verified <url> <date>`, `unverified`, or `no source`."""
    source = table.get("source") if _is_record(table) else None
    if source == Ident("Unverified"):
        return "unverified"
    if isinstance(source, Tagged) and source.name == "Verified" and _is_record(source.args):
        return f"verified {source.args.get('url', '?')} {source.args.get('verified', '?')}"
    return "no source" if source is None else "invalid source"


def protocols_text(table: object) -> str:
    protocols = table.get("protocols") if _is_record(table) else None
    if isinstance(protocols, list):
        return "[" + ", ".join(str(p) for p in protocols) + "]"
    return "[?]"


class MapInfo:
    """What the model map says about each file."""

    def __init__(self) -> None:
        # stem -> list of (model number, list key) it appears under
        self.refs: dict[str, list[tuple[int, str]]] = {}
        # stem -> not_mapped reason text
        self.not_mapped: dict[str, str] = {}
        # model number -> entry record (well-formed entries only)
        self.models: dict[int, dict] = {}

    def mapped(self, stem: str) -> bool:
        return stem in self.refs

    def under(self, stem: str, key: str) -> bool:
        return any(k == key for _n, k in self.refs.get(stem, []))

    def mapping_text(self, stem: str) -> str:
        if stem in self.refs:
            numbers: dict[int, list[str]] = {}
            for number, key in self.refs[stem]:
                numbers.setdefault(number, [])
                if key not in numbers[number]:
                    numbers[number].append(key)
            parts = []
            for number, keys in numbers.items():
                shown = [k for k in ("p1", "p2") if k in keys] or ["tables"]
                parts.append(" ".join([str(number), *shown]))
            return ", ".join(parts)
        if stem in self.not_mapped:
            return f"not mapped: {self.not_mapped[stem]}"
        return "not mapped"


LIST_KEYS = ("p1", "p2", "tables")


def read_map(model_map: object, report: Report) -> MapInfo:
    """Collects which file sits under which model number and list; reports shape errors."""
    info = MapInfo()
    if not _is_record(model_map):
        report.add(MAP_FILE, "", "the map is not a record `( models: { … } )`")
        return info
    models = model_map.get("models")
    if not isinstance(models, dict) or any(not isinstance(k, int) for k in models):
        report.add(MAP_FILE, "`models`", "must be a map `{ NUMBER: ( … ) }`")
        return info
    for number, entry in models.items():
        if not 0 <= number <= 0xFFFF:
            report.add(MAP_FILE, f"model {number}", "model number must be 0–65535")
        if not _is_record(entry):
            report.add(MAP_FILE, f"model {number}", "entry must be a record `( … )`")
            continue
        info.models[number] = entry
        for key in LIST_KEYS:
            refs = entry.get(key)
            if not isinstance(refs, list):
                continue
            for ref in refs:
                if _is_record(ref) and isinstance(ref.get("file"), str):
                    info.refs.setdefault(ref["file"], []).append((number, key))
    return info


# `rx` joined the families on 2026-10-28 with RX-28 (spec Clarifications, module-interfaces §9).
FAMILIES = ("ax", "mx", "rx", "x-series", "y-series", "pro")
RUST_KEYWORDS = frozenset(
    "as async await break const continue crate dyn else enum extern false fn for gen if impl in let loop "
    "match mod move mut pub ref return self Self static struct super trait true type unsafe use where while "
    "abstract become box do final macro override priv try typeof unsized virtual yield".split()
)
DIGITS = ("zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine")


def module_ident(stem: str) -> str:
    """`2xl430_w250` -> `two_xl430_w250`, as xtask names the generated module."""
    if stem[:1].isdigit():
        return f"{DIGITS[int(stem[0])]}_{stem[1:]}"
    return stem


def type_ident(stem: str) -> str:
    """`2xl430_w250` -> `TwoXl430W250`, as xtask names the generated type."""
    return "".join(part[:1].upper() + part[1:] for part in module_ident(stem).split("_"))


def _refs(entry: dict, key: str, loc: str, report: Report) -> list[tuple[int, str]] | None:
    """A `p1`/`p2`/`tables` list as (from_fw, file) pairs, or None when malformed."""
    refs = entry.get(key)
    if not isinstance(refs, list):
        report.add(MAP_FILE, loc, f"`{key}` must be a list of `(from_fw: …, file: …)`")
        return None
    out = []
    for ref in refs:
        if not (_is_record(ref) and isinstance(ref.get("from_fw"), int) and isinstance(ref.get("file"), str)):
            report.add(MAP_FILE, loc, f"`{key}`: {ref!r} is not `(from_fw: INT, file: STRING)`")
            return None
        out.append((ref["from_fw"], ref["file"]))
    return out


def check_map(tables: dict[str, object], info: MapInfo, report: Report) -> None:
    """Model map rules (data-model "Model entry", "Table ref", "Map validation"; ron-schema §2)."""
    for number, entry in sorted(info.models.items()):
        loc = f"model {number}"
        family = entry.get("family")
        if family not in FAMILIES:
            report.add(MAP_FILE, loc, f"`family` must be one of {', '.join(FAMILIES)}, found {family!r}")
        if not isinstance(entry.get("name"), str) or not entry.get("name"):
            report.add(MAP_FILE, loc, "`name` must be a non-empty string")
        lists: dict[str, list[tuple[int, str]]] = {}
        for key in ("p1", "p2"):
            if key in entry:
                refs = _refs(entry, key, loc, report)
                if refs is not None:
                    lists[key] = refs
        if "p1" not in entry and "p2" not in entry:
            report.add(MAP_FILE, loc, "no `p1` or `p2` list")
        for key, refs in lists.items():
            protocol = key.upper()
            if not refs:
                report.add(MAP_FILE, loc, f"`{key}` is empty")
                continue
            if refs[0][0] != 0:
                report.add(MAP_FILE, loc, f"`{key}`: the first `from_fw` must be 0, found {refs[0][0]}")
            if any(not 0 <= fw <= 255 for fw, _f in refs):
                report.add(MAP_FILE, loc, f"`{key}`: `from_fw` must be 0–255")
            if any(a[0] >= b[0] for a, b in zip(refs, refs[1:])):
                report.add(MAP_FILE, loc, f"`{key}`: `from_fw` must strictly increase")
            for _fw, stem in refs:
                if stem not in tables:
                    report.add(MAP_FILE, loc, f"`{key}` names {stem}.ron, which does not exist")
                    continue
                names = _protocol_names(tables[stem]) if _is_record(tables[stem]) else None
                if names is not None and protocol not in names:
                    report.add(MAP_FILE, loc, f"`{key}` names {stem}.ron, whose `protocols` lack {protocol}")
            places = set()
            for _fw, stem in refs:
                table = tables.get(stem)
                for row in table_rows(table):
                    if row["data_name"] == "Firmware Version":
                        places.add((row["address"], row["size"]))
            if len(places) > 1:
                shown = ", ".join(f"({a}, {s})" for a, s in sorted(places))
                report.add(MAP_FILE, loc, f"`{key}`: `Firmware Version` sits at different places: {shown}")
        both = "p1" in entry and "p2" in entry
        default = entry.get("default")
        if both and default is None:
            report.add(MAP_FILE, loc, "`default: P2` is required when both `p1` and `p2` exist")
        elif both and default != Ident("P2"):
            report.add(MAP_FILE, loc, f"`default` must be P2, found {default}")
        elif not both and default is not None:
            report.add(MAP_FILE, loc, "`default` is allowed only when both `p1` and `p2` exist")
        listed = {stem for refs in lists.values() for _fw, stem in refs}
        for stem in sorted(listed):
            names = _protocol_names(tables[stem]) if _is_record(tables.get(stem)) else None
            if names and "P1" in names and "P2" in names and not both:
                report.add(
                    MAP_FILE, loc, f"{stem}.ron lists both protocols, so the model needs both `p1` and `p2` lists"
                )
        if "tables" not in entry:
            report.add(MAP_FILE, loc, "missing the legacy `tables` list (T1: equal to the default or only list)")
        else:
            legacy = _refs(entry, "tables", loc, report)
            expected = lists.get("p2") if both else next(iter(lists.values()), None)
            if legacy is not None and expected is not None and legacy != expected:
                report.add(MAP_FILE, loc, "legacy `tables` differs from the default (or only) list")
    for stem, places in sorted(info.refs.items()):
        numbers = sorted({n for n, _k in places})
        if len(numbers) > 1:
            report.add(
                MAP_FILE,
                "",
                f"{stem}.ron is mapped under model numbers {' and '.join(map(str, numbers))}; a file sits under one number",
            )
    seen_types: dict[str, str] = {}
    for stem in sorted(info.refs):
        ident = type_ident(stem)
        if ident in seen_types:
            report.add(MAP_FILE, "", f"{seen_types[ident]}.ron and {stem}.ron both give the type `{ident}`")
        else:
            seen_types[ident] = stem
        module = module_ident(stem)
        if module == "tests" or module in RUST_KEYWORDS:
            report.add(MAP_FILE, "", f"{stem}.ron gives the module name `{module}`, which is `tests` or a Rust keyword")


def check_not_mapped(model_map: object, tables: dict[str, object], info: MapInfo, report: Report) -> None:
    """Every table is mapped or in `not_mapped`, never both, never neither (FR-011, xtask checks 6–7)."""
    records = model_map.get("not_mapped", []) if _is_record(model_map) else []
    if not isinstance(records, list):
        report.add(MAP_FILE, "`not_mapped`", "must be a list of `(file: …, reason: …)`")
        records = []
    for i, record in enumerate(records):
        loc = f"`not_mapped` item {i}"
        if not (_is_record(record) and isinstance(record.get("file"), str)):
            report.add(MAP_FILE, loc, "must be `(file: STRING, reason: NoModelNumber | SameNumberAs(N))`")
            continue
        stem = record["file"]
        loc = f"`not_mapped` {stem}"
        reason = record.get("reason")
        if reason == Ident("NoModelNumber"):
            text = "no model number"
        elif (
            isinstance(reason, Tagged)
            and reason.name == "SameNumberAs"
            and isinstance(reason.args, list)
            and len(reason.args) == 1
            and isinstance(reason.args[0], int)
            and 0 <= reason.args[0] <= 0xFFFF
        ):
            number = reason.args[0]
            text = f"same number as {number}"
            if number not in info.models:
                report.add(MAP_FILE, loc, f"SameNumberAs({number}) names a number that is not in `models`")
        else:
            report.add(
                MAP_FILE, loc, f"reason must be NoModelNumber or SameNumberAs(N), found {reason!r} (unverified is never a reason)"
            )
            continue
        if stem not in tables:
            report.add(MAP_FILE, loc, f"{stem}.ron does not exist")
        elif info.mapped(stem):
            report.add(MAP_FILE, loc, f"{stem}.ron is mapped and also in `not_mapped`")
        if stem in info.not_mapped:
            report.add(MAP_FILE, loc, f"{stem} is listed twice in `not_mapped`")
        info.not_mapped.setdefault(stem, text)
    for stem in tables:
        if not info.mapped(stem) and stem not in info.not_mapped:
            report.add(f"{stem}.ron", "", "not mapped and not in `not_mapped` of dynamixel.ron")


def run_checks(model_map: object, tables: dict[str, object], today: datetime.date) -> tuple[Report, MapInfo]:
    """Runs every rule; returns the errors and the map summary."""
    report = Report()
    info = read_map(model_map, report)
    for stem, table in tables.items():
        check_table_shape(stem, table, report)
        if not _is_record(table):
            continue
        check_source(stem, table, today, report)
        check_protocols(stem, table, report)
        check_instructions(stem, table, report)
        check_row_fields(stem, table, report)
        check_mode_source(stem, table, report)
        if info.mapped(stem):
            check_roles(stem, table, info, report)
    check_map(tables, info, report)
    check_not_mapped(model_map, tables, info, report)
    return report, info


def dual_protocol_lines(info: MapInfo) -> list[str]:
    lines = []
    for number, entry in sorted(info.models.items()):
        p1, p2 = entry.get("p1"), entry.get("p2")
        if not (isinstance(p1, list) and isinstance(p2, list)):
            continue

        def files(refs: list) -> str:
            return ", ".join(r["file"] for r in refs if _is_record(r) and isinstance(r.get("file"), str))

        default = entry.get("default")
        default_text = f"default: {default}" if default is not None else "default: missing"
        lines.append(f"{number} {entry.get('name', '?')}   p1: {files(p1)}   p2: {files(p2)}   {default_text}")
    return lines


def render(
    tables: dict[str, object],
    selected: list[str],
    report: Report,
    info: MapInfo,
    markdown: bool,
) -> str:
    rows = [(stem, flag_text(tables[stem]), protocols_text(tables[stem]), info.mapping_text(stem)) for stem in selected]
    if markdown:
        return "".join(f"- `{stem}`: {flag}, {protocols}, {mapping}\n" for stem, flag, protocols, mapping in rows)
    out = ["tables"]
    if rows:
        widths = [max(len(r[i]) for r in rows) for i in range(3)]
        for r in rows:
            out.append("  " + "  ".join(r[i].ljust(widths[i]) for i in range(3)) + "  " + r[3])
    out.append("dual-protocol models")
    duals = dual_protocol_lines(info)
    out.extend(f"  {line}" for line in duals)
    out.append("errors")
    out.extend(f"  {line}" for line in report.lines())
    verified = sum(1 for t in tables.values() if flag_text(t).startswith("verified "))
    unverified = sum(1 for t in tables.values() if flag_text(t) == "unverified")
    mapped = sum(1 for stem in tables if info.mapped(stem))
    out.append(
        f"summary: {len(tables)} tables, {verified} verified, {unverified} unverified, "
        f"{mapped} mapped, {len(tables) - mapped} not mapped, {len(duals)} dual-protocol, "
        f"{len(report.errors)} errors"
    )
    return "\n".join(out) + "\n"


def _iso_date(text: str) -> datetime.date:
    if not DATE_RE.match(text):
        raise argparse.ArgumentTypeError(f"not an ISO date YYYY-MM-DD: {text!r}")
    try:
        return datetime.date.fromisoformat(text)
    except ValueError as err:
        raise argparse.ArgumentTypeError(f"not a real date: {text!r} ({err})") from None


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="check_tables.py",
        description="Check the control tables and the model map (no network).",
    )
    parser.add_argument(
        "--dir",
        type=Path,
        default=Path(__file__).resolve().parent,
        help="directory holding the tables and dynamixel.ron (default: this script's directory)",
    )
    parser.add_argument(
        "--today",
        type=_iso_date,
        default=None,
        help="the 'not in the future' bound for verified dates (default: today)",
    )
    parser.add_argument(
        "--markdown",
        action="store_true",
        help="print the per-table lines as a Markdown list",
    )
    parser.add_argument("stems", nargs="*", metavar="STEM", help="limit the per-table section")
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    try:
        args = parser.parse_args(argv)
    except SystemExit as exc:
        return int(exc.code or 0)
    if args.today is None:
        args.today = datetime.date.today()
    try:
        model_map, tables = load(args.dir)
    except (LoadError, RonError, OSError) as err:
        print(f"error: {err}", file=sys.stderr)
        return 2
    unknown = [stem for stem in args.stems if stem not in tables]
    if unknown:
        print(f"error: no table named {', '.join(unknown)} in {args.dir}", file=sys.stderr)
        return 2
    report, info = run_checks(model_map, tables, args.today)
    selected = args.stems or list(tables)
    sys.stdout.write(render(tables, selected, report, info, args.markdown))
    for line in report.lines():
        print(line, file=sys.stderr)
    return 1 if report.errors else 0


if __name__ == "__main__":
    sys.exit(main())

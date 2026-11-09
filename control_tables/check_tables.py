#!/usr/bin/env python3
"""Table check for `control_tables/`: validates every table and `dynamixel.ron`.

Command line, output and exit codes are the contract in
`specs/005-tables/contracts/table-tools.md` section 1, made precise by SRS
FR-6.14; the rules are those of `specs/005-tables/contracts/ron-schema.md`
sections 1 and 2, extended for the FR-9 register data by
`specs/009-tables-fr9/contracts/fr9-schema.md` (see "FR-9 register data" below
for the vocabulary).

    python3 control_tables/check_tables.py [--dir DIR] [--today YYYY-MM-DD] [--markdown] [STEM ...]

Every finding is a `Diagnostic`: a cause code from `CAUSES`, the file, the
location in it, the model number, register and field involved, and the values
that decided it (`found` against `expected` or `allowed`, and any other), next
to a message that states those facts. It prints as one line,

    FILE: LOCATION: [CAUSE] MESSAGE

followed, when the cause has advice, by an indented `hint:` line. A cause code
is stable: once published it keeps its meaning; a new check gets a new code
and a removed check retires its code for good.

Exit codes, one per failure class (the highest wins when several occur):

    0  no diagnostic
    1  a rule is broken
    2  bad arguments, a file that does not parse or a missing `dynamixel.ron`

No network.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime
import re
import sys
from pathlib import Path
from typing import NamedTuple

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
# Diagnostics (SRS FR-6.14)
# --------------------------------------------------------------------------

PROG = "check_tables.py"
EXIT_OK = 0
EXIT_RULE = 1
EXIT_LOAD = 2


class Cause(NamedTuple):
    """What a cause code means for a run: its exit code, and the advice it carries, if any."""

    exit_code: int
    hint: str | None = None


RULE = Cause(EXIT_RULE)
FR9_HINT = (
    "the FR-9 data is verified per table: add `unit_source: Verified(url: …, verified: …)` "
    "once the e-Manual page is checked"
)

# Every cause code the check emits. A code is stable: never renamed, never reused.
# DECISION: codes are kebab-case names (`row-size-invalid`), not numbers, so a line reads without a lookup
# table; one code per check, and a "must be a non-empty list" check keeps one `-shape` code whose `found`
# tells an empty list from another value; advice is static per code, as `Error::hint()` is (FR-6.12).
CAUSES: dict[str, Cause] = {
    # table shape
    "table-not-record": RULE,
    "control-table-missing": RULE,
    "control-table-shape": RULE,
    "row-not-record": RULE,
    "row-address-invalid": RULE,
    "row-size-invalid": RULE,
    "row-name-invalid": RULE,
    # source and URLs
    "source-missing": RULE,
    "source-shape": RULE,
    "source-unknown-field": RULE,
    "source-url-missing": RULE,
    "source-date-missing": RULE,
    "source-date-not-iso": RULE,
    "source-date-not-real": RULE,
    "source-date-future": Cause(
        EXIT_RULE, "the date is the day the e-Manual page was checked; `--today` sets the bound"
    ),
    "url-not-string": RULE,
    "url-not-emanual": Cause(EXIT_RULE, "the e-Manual is the only source (Principle IV)"),
    # instructions
    "instructions-missing": Cause(
        EXIT_RULE, "a verified table lists the instructions of each of its protocols, from the e-Manual protocol page"
    ),
    "instructions-on-unverified": RULE,
    "instructions-shape": RULE,
    "instruction-entry-not-record": RULE,
    "instruction-entry-unknown-field": RULE,
    "instruction-protocol-invalid": RULE,
    "instruction-protocol-not-in-table": RULE,
    "instruction-source-missing": RULE,
    "instruction-list-shape": RULE,
    "instruction-unknown": RULE,
    "instruction-duplicate": RULE,
    "instruction-not-p1": RULE,
    "instruction-from-fw-invalid": RULE,
    "instruction-from-fw-duplicate": RULE,
    "instructions-protocol-missing": RULE,
    # protocols
    "protocols-missing": RULE,
    "protocols-not-list": RULE,
    "protocols-empty": RULE,
    "protocol-unknown": RULE,
    "protocol-duplicate": RULE,
    # row fields
    "row-access-missing": RULE,
    "row-access-invalid": RULE,
    "row-area-missing": RULE,
    "row-area-invalid": RULE,
    "row-unknown-field": RULE,
    # mode source
    "mode-row-duplicate": RULE,
    "mode-rows-both": RULE,
    "mode-row-beside-angle-limits": RULE,
    "control-mode-outside-xl320": Cause(
        EXIT_RULE,
        "only xl320.ron carries a `Control Mode` row (contract 4.3); a second such table goes to the architect first",
    ),
    "mode-source-missing": Cause(
        EXIT_RULE, "a table has an `Operating Mode` or `Control Mode` row, or the CW/CCW Angle Limit pair (FR-7.6)"
    ),
    "mode-row-attributes": RULE,
    "operating-modes-missing": RULE,
    "operating-modes-without-mode-row": RULE,
    # operating_modes
    "operating-modes-not-record": RULE,
    "operating-modes-key-missing": RULE,
    "operating-modes-unknown-key": RULE,
    "per-mode-lists-need-unit-source": Cause(EXIT_RULE, FR9_HINT),
    "per-mode-list-missing": Cause(
        EXIT_RULE, "the per-mode lists `current`, `velocity`, `pwm` and `current_position` come all four or none"
    ),
    "mode-list-not-list": RULE,
    "mode-value-invalid": RULE,
    "per-mode-value-outside-parent": RULE,
    "per-mode-value-twice": RULE,
    "per-mode-value-uncovered": RULE,
    "mode-value-twice": RULE,
    # roles (mapped tables)
    "role-missing": RULE,
    "role-attributes": RULE,
    "model-number-place": RULE,
    "limit-pair-half": RULE,
    "limit-forms-both": RULE,
    "limit-pair-missing": RULE,
    "limit-attributes": RULE,
    "p1-address-above-255": Cause(EXIT_RULE, "Protocol 1 addresses are one byte"),
    "register-constant-duplicate": RULE,
    # type_info
    "type-info-missing": RULE,
    "type-info-not-record": RULE,
    "type-info-key-missing": RULE,
    "type-info-position-not-int": RULE,
    "type-info-radian-not-number": RULE,
    "type-info-max-not-above-min": RULE,
    "type-info-max-radian-not-above-min": RULE,
    "type-info-zero-outside": RULE,
    # ranges
    "range-shape": RULE,
    "range-not-int": RULE,
    "range-min-above-max": RULE,
    "range-does-not-fit": RULE,
    # unit_info
    "unit-info-not-list": RULE,
    "unit-entry-not-record": RULE,
    "unit-entry-name-invalid": RULE,
    "unit-entry-key-missing": RULE,
    "unit-entry-value-invalid": RULE,
    "unit-entry-unit-invalid": RULE,
    "unit-entry-fr9-needs-unit-source": Cause(EXIT_RULE, FR9_HINT),
    "unit-entry-unknown-field": RULE,
    "unit-entry-sign-unknown": RULE,
    "unit-entry-no-row": RULE,
    "unit-entry-unit-unknown": RULE,
    "unit-entry-sign-invalid": RULE,
    "unit-entry-direction-bit-outside": RULE,
    "unit-entry-modes-need-angle-limits": RULE,
    "unit-entry-modes-invalid": RULE,
    "unit-entry-mode-unknown": RULE,
    "unit-entry-mode-needs-torque-control": RULE,
    "unit-entry-mode-duplicate": RULE,
    "unit-entry-profile-invalid": RULE,
    "unit-entry-profile-needs-drive-mode": RULE,
    "unit-entry-zero-is-unlimited-invalid": RULE,
    "unit-entry-from-fw-invalid": RULE,
    "unit-entry-overlap": Cause(EXIT_RULE, "an entry without `modes` or `profile` applies in all of them"),
    "bounded-by-no-row": RULE,
    "bounded-by-self": RULE,
    "bounded-by-scale-mismatch": RULE,
    "row-range-needs-unit-source": Cause(EXIT_RULE, FR9_HINT),
    # torque_control_mode
    "torque-control-needs-unit-source": Cause(EXIT_RULE, FR9_HINT),
    "torque-control-needs-angle-limits": RULE,
    "torque-control-not-record": RULE,
    "torque-control-key-missing": RULE,
    "torque-control-unknown-key": RULE,
    "torque-control-on-invalid": RULE,
    "torque-control-no-row": RULE,
    "torque-control-row-attributes": RULE,
    "torque-control-on-outside-range": RULE,
    "torque-control-same-row": RULE,
    # model map
    "map-not-record": RULE,
    "models-not-map": RULE,
    "model-number-invalid": RULE,
    "model-entry-not-record": RULE,
    "model-family-invalid": RULE,
    "model-name-invalid": RULE,
    "model-also-shape": RULE,
    "model-also-names": RULE,
    "model-list-not-list": RULE,
    "model-ref-invalid": RULE,
    "model-lists-missing": RULE,
    "model-list-empty": RULE,
    "model-first-from-fw": RULE,
    "model-from-fw-range": RULE,
    "model-from-fw-order": RULE,
    "model-file-missing": RULE,
    "model-file-protocol": RULE,
    "model-firmware-version-places": RULE,
    "model-default-missing": RULE,
    "model-default-invalid": RULE,
    "model-default-unexpected": RULE,
    "model-needs-both-lists": RULE,
    "model-legacy-tables": Cause(
        EXIT_RULE, "the `tables` key was removed in T2; list the tables under `p1` and `p2` (ron-schema §2)"
    ),
    "file-under-two-numbers": RULE,
    "type-ident-duplicate": RULE,
    "module-ident-reserved": RULE,
    # not_mapped
    "not-mapped-not-list": RULE,
    "not-mapped-record-invalid": RULE,
    "not-mapped-number-unknown": RULE,
    "not-mapped-reason-invalid": Cause(
        EXIT_RULE, "being unverified is never a reason: an unverified table stays mapped and flagged unverified"
    ),
    "not-mapped-file-missing": RULE,
    "not-mapped-also-mapped": RULE,
    "not-mapped-duplicate": RULE,
    "table-unlisted": Cause(
        EXIT_RULE, "map it under its model number, or record it in `not_mapped` with its reason (FR-7.4)"
    ),
}


@dataclasses.dataclass
class Diagnostic:
    """One finding: a stable cause code, its facts as fields, and a message that states them.

    `file` is the file the finding is in (empty for the command line) and
    `location` the row, key or model number inside it; `model`, `register`
    and `field` name the model number, register and key involved; `values`
    holds the values that decided it (`found` against `expected` or
    `allowed`, and any other, such as `index`, `table` or `today`).
    """

    code: str
    file: str
    location: str
    message: str
    model: int | None = None
    register: str | None = None
    field: str | None = None
    values: dict[str, object] = dataclasses.field(default_factory=dict)

    @property
    def exit_code(self) -> int:
        return CAUSES[self.code].exit_code

    @property
    def hint(self) -> str | None:
        """Advice for the usual cause, kept apart from the facts of the message (FR-6.12)."""
        return CAUSES[self.code].hint

    def data(self) -> dict[str, object]:
        """Every field as one flat dict."""
        return {
            "code": self.code,
            "file": self.file,
            "location": self.location,
            "message": self.message,
            "model": self.model,
            "register": self.register,
            "field": self.field,
            **self.values,
        }

    def line(self) -> str:
        """The printed line: `FILE: LOCATION: [CAUSE] MESSAGE`."""
        # DECISION: keep the old `file: location: message` frame and insert `[cause]`; no script, xtask step or
        # CI job parses the text (CI reads only the exit code), and grep on `[cause]` finds every occurrence.
        where = self.file or PROG
        if "line" in self.values and "column" in self.values:
            where += f":{self.values['line']}:{self.values['column']}"
        head = f"{where}: {self.location}: " if self.location else f"{where}: "
        return f"{head}[{self.code}] {self.message}"


class Report:
    """The diagnostics of one run."""

    def __init__(self) -> None:
        self.diagnostics: list[Diagnostic] = []

    def add(
        self,
        code: str,
        file: str,
        location: str,
        message: str,
        *,
        model: int | None = None,
        register: str | None = None,
        field: str | None = None,
        **values: object,
    ) -> None:
        """Records one diagnostic. A code missing from `CAUSES` is a bug in this script: KeyError."""
        if code not in CAUSES:
            raise KeyError(f"unregistered cause code {code!r}")
        self.diagnostics.append(Diagnostic(code, file, location, message, model, register, field, values))

    def lines(self) -> list[str]:
        """One line per diagnostic, each followed by `  hint: …` when its cause has advice."""
        out: list[str] = []
        # DECISION: the hint goes on its own indented line under its diagnostic, on stdout and stderr alike,
        # so the diagnostic line itself holds facts only (FR-6.12) and stays one line per finding.
        for diagnostic in self.diagnostics:
            out.append(diagnostic.line())
            if diagnostic.hint:
                out.append(f"  hint: {diagnostic.hint}")
        return out

    def exit_code(self) -> int:
        """The highest exit code among the diagnostics, 0 without any."""
        # DECISION: when several failure classes occur the highest code wins; every class above 1 is input the
        # check could not take in, which outranks a broken rule.
        return max((d.exit_code for d in self.diagnostics), default=EXIT_OK)


def _describe(value: object) -> str:
    """A parsed value in a few words, for a message that must not print a whole table."""
    # DECISION: a shape check (record, list) reports this description as `found`, not the raw value, which
    # can be a whole table; a value check reports the raw value itself.
    if value is None:
        return "nothing"
    if isinstance(value, Record):
        return "a record" if value else "an empty record"
    if isinstance(value, dict):
        return "a map"
    if isinstance(value, list):
        return "a list" if value else "an empty list"
    if isinstance(value, str):
        return f"string {value!r}"
    if isinstance(value, int):
        return f"integer {value}"
    if isinstance(value, float):
        return f"number {value}"
    if isinstance(value, Ident):
        return f"identifier `{value.name}`"
    if isinstance(value, Tagged):
        return f"`{value.name}(…)`"
    return type(value).__name__


def _name(value: object) -> object:
    """An identifier's name (`RW` for `Ident("RW")`), any other value as it is."""
    return value.name if isinstance(value, Ident) else value


def _shown(value: object) -> str:
    """`_name(value)` for a message, `-` when absent."""
    return "-" if value is None else str(_name(value))


# --------------------------------------------------------------------------
# Checking
# --------------------------------------------------------------------------


def _is_record(value: object) -> bool:
    return isinstance(value, dict) and all(isinstance(k, str) for k in value)


def _row_location(row: dict) -> str:
    address = row.get("address", "?")
    name = row.get("data_name", "?")
    return f'row {address} "{name}"'


def _register(row: dict) -> str | None:
    """The row's `data_name` when it is a string."""
    name = row.get("data_name")
    return name if isinstance(name, str) else None


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
        found = _describe(table)
        report.add("table-not-record", file, "", f"the file is not a record `( … )`, found {found}", found=found)
        return
    rows = table.get("control_table")
    if rows is None:
        report.add("control-table-missing", file, "", "missing `control_table`", field="control_table")
        return
    if not isinstance(rows, list) or not rows:
        found = _describe(rows)
        report.add(
            "control-table-shape",
            file,
            "`control_table`",
            f"must be a non-empty list of rows, found {found}",
            field="control_table",
            found=found,
        )
        return
    for i, row in enumerate(rows):
        if not _is_record(row):
            found = _describe(row)
            report.add(
                "row-not-record",
                file,
                f"`control_table` item {i}",
                f"not a row record, found {found}",
                field="control_table",
                index=i,
                found=found,
            )
            continue
        loc = _row_location(row)
        register = _register(row)
        address, size, name = row.get("address"), row.get("size"), row.get("data_name")
        if not _is_int(address) or not 0 <= address <= 0xFFFF:
            report.add(
                "row-address-invalid",
                file,
                loc,
                f"`address` must be an integer 0–65535, found {address!r}",
                register=register,
                field="address",
                found=address,
                allowed=(0, 0xFFFF),
            )
        if not _is_int(size) or not 1 <= size <= 0xFF:
            report.add(
                "row-size-invalid",
                file,
                loc,
                f"`size` must be an integer 1–255, found {size!r}",
                register=register,
                field="size",
                found=size,
                allowed=(1, 0xFF),
            )
        if not isinstance(name, str) or not name:
            report.add(
                "row-name-invalid",
                file,
                loc,
                f"`data_name` must be a non-empty string, found {name!r}",
                register=register,
                field="data_name",
                found=name,
            )


def check_url(file: str, location: str, url: object, report: Report, field: str, **context: object) -> None:
    """An e-Manual URL: a string starting with `https://emanual.robotis.com/`."""
    if not isinstance(url, str):
        report.add(
            "url-not-string", file, location, f"must be a URL string, found {url!r}", field=field, found=url, **context
        )
    elif not url.startswith(EMANUAL_PREFIX):
        report.add(
            "url-not-emanual",
            file,
            location,
            f"URL {url!r} is not on {EMANUAL_PREFIX}",
            field=field,
            found=url,
            expected=EMANUAL_PREFIX,
            **context,
        )


def check_source(stem: str, table: dict, today: datetime.date, report: Report) -> None:
    """`source: Unverified` or `source: Verified(url, verified)` (FR-001, FR-002)."""
    file = f"{stem}.ron"
    expected = "`Unverified` or `Verified(url: …, verified: …)`"
    if "source" not in table:
        report.add(
            "source-missing", file, "", f"missing `source`, expected {expected}", field="source", expected=expected
        )
        return
    source = table["source"]
    if source == Ident("Unverified"):
        return
    check_verified(file, "source", source, today, report, expected)


def check_verified(file: str, key: str, value: object, today: datetime.date, report: Report, expected: str) -> None:
    """A `Verified(url: …, verified: …)` value under `key`: e-Manual URL, real ISO date, not in the future."""
    loc = f"`{key}`"
    if not (isinstance(value, Tagged) and value.name == "Verified" and _is_record(value.args)):
        report.add(
            "source-shape", file, loc, f"must be {expected}, found {value!r}", field=key, found=value, expected=expected
        )
        return
    args = value.args
    extra = sorted(set(args) - {"url", "verified"})
    if extra:
        report.add("source-unknown-field", file, loc, f"unknown field(s) {', '.join(extra)}", field=key, found=extra)
    if "url" not in args:
        report.add("source-url-missing", file, loc, "missing `url`", field=f"{key}.url")
    else:
        check_url(file, f"`{key}` url", args["url"], report, f"{key}.url")
    date = args.get("verified")
    field = f"{key}.verified"
    if date is None:
        report.add("source-date-missing", file, loc, "missing `verified` date", field=field)
    elif not isinstance(date, str) or not DATE_RE.match(date):
        report.add(
            "source-date-not-iso",
            file,
            f"`{key}` verified",
            f"{date!r} is not an ISO date YYYY-MM-DD",
            field=field,
            found=date,
        )
    else:
        try:
            day = datetime.date.fromisoformat(date)
        except ValueError:
            report.add(
                "source-date-not-real",
                file,
                f"`{key}` verified",
                f"{date!r} is not a real date",
                field=field,
                found=date,
            )
        else:
            if day > today:
                report.add(
                    "source-date-future",
                    file,
                    f"`{key}` verified",
                    f"{date} is in the future (today is {today.isoformat()})",
                    field=field,
                    found=date,
                    today=today.isoformat(),
                )


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
    source = table.get("source")
    is_verified = isinstance(source, Tagged)
    entries = table.get("instructions")
    if entries is None:
        if is_verified:
            report.add(
                "instructions-missing", file, "`instructions`", "missing on a verified table", field="instructions"
            )
        return
    if not is_verified:
        report.add(
            "instructions-on-unverified",
            file,
            "`instructions`",
            f"present while `source` is {_shown(source)}",
            field="instructions",
            source=source,
        )
        return
    if not isinstance(entries, list) or not entries:
        found = _describe(entries)
        report.add(
            "instructions-shape",
            file,
            "`instructions`",
            f"must be a non-empty list of entries, found {found}",
            field="instructions",
            found=found,
        )
        return
    protocols = _protocol_names(table)
    seen: dict[str, list[object]] = {}
    for i, entry in enumerate(entries):
        loc = f"`instructions` entry {i}"
        if not _is_record(entry):
            found = _describe(entry)
            report.add(
                "instruction-entry-not-record",
                file,
                loc,
                f"must be a record `(protocol: …, source: …, list: […])`, found {found}",
                field="instructions",
                index=i,
                found=found,
            )
            continue
        unknown = sorted(set(entry) - {"protocol", "source", "list", "from_fw"})
        if unknown:
            report.add(
                "instruction-entry-unknown-field",
                file,
                loc,
                f"unknown field(s) {', '.join(unknown)}",
                field="instructions",
                index=i,
                found=unknown,
            )
        protocol = entry.get("protocol")
        name = protocol.name if isinstance(protocol, Ident) else None
        if name not in PROTOCOLS:
            report.add(
                "instruction-protocol-invalid",
                file,
                loc,
                f"`protocol` must be P1 or P2, found {protocol!r}",
                field="protocol",
                index=i,
                found=protocol,
                allowed=PROTOCOLS,
            )
        elif protocols is not None and name not in protocols:
            report.add(
                "instruction-protocol-not-in-table",
                file,
                loc,
                f"`protocol` {name} is not in the table's `protocols` [{', '.join(protocols)}]",
                field="protocol",
                index=i,
                found=name,
                allowed=protocols,
            )
        if "source" not in entry:
            report.add(
                "instruction-source-missing",
                file,
                loc,
                "missing `source` (the protocol page URL)",
                field="source",
                index=i,
            )
        else:
            check_url(file, f"{loc} `source`", entry["source"], report, "instructions.source", index=i)
        names = entry.get("list")
        if not isinstance(names, list) or not names:
            found = _describe(names)
            report.add(
                "instruction-list-shape",
                file,
                loc,
                f"`list` must be a non-empty list of instruction names, found {found}",
                field="list",
                index=i,
                found=found,
            )
        else:
            listed: list[str] = []
            for item in names:
                if not isinstance(item, Ident) or item.name not in INSTRUCTIONS:
                    report.add(
                        "instruction-unknown",
                        file,
                        loc,
                        f"unknown instruction {item!r}",
                        field="list",
                        index=i,
                        found=item,
                        allowed=INSTRUCTIONS,
                    )
                    continue
                if item.name in listed:
                    report.add(
                        "instruction-duplicate",
                        file,
                        loc,
                        f"{item.name} listed twice",
                        field="list",
                        index=i,
                        found=item.name,
                    )
                listed.append(item.name)
                if name == "P1" and item.name not in P1_INSTRUCTIONS:
                    report.add(
                        "instruction-not-p1",
                        file,
                        loc,
                        f"{item.name} is not a Protocol 1.0 instruction",
                        field="list",
                        index=i,
                        found=item.name,
                        allowed=P1_INSTRUCTIONS,
                    )
        from_fw = entry.get("from_fw")
        if from_fw is not None and (not _is_int(from_fw) or not 0 <= from_fw <= 255):
            report.add(
                "instruction-from-fw-invalid",
                file,
                loc,
                f"`from_fw` must be an integer 0–255, found {from_fw!r}",
                field="from_fw",
                index=i,
                found=from_fw,
                allowed=(0, 255),
            )
        if name in PROTOCOLS:
            if from_fw in seen.setdefault(name, []):
                shown = "none" if from_fw is None else from_fw
                report.add(
                    "instruction-from-fw-duplicate",
                    file,
                    loc,
                    f"two {name} entries with the same `from_fw` ({shown})",
                    field="from_fw",
                    index=i,
                    protocol=name,
                    found=from_fw,
                )
            seen[name].append(from_fw)
    for protocol in protocols or []:
        if protocol in PROTOCOLS and protocol not in seen:
            report.add(
                "instructions-protocol-missing",
                file,
                "`instructions`",
                f"no entry for {protocol}, which is in `protocols`",
                field="instructions",
                protocol=protocol,
            )


ACCESS_VALUES = ("R", "RW")
# `Hybrid` (Y series): architect 2026-10-29 (provisional), data-model "Register row".
AREA_VALUES = ("EEPROM", "RAM", "Hybrid")
MODE_NAMES = ("Operating Mode", "Control Mode")
MODE_LISTS = ("bounded", "unbounded", "no_position")
# FR-9 per-mode lists (specs/009-tables-fr9): optional, all four or none, only with
# `unit_source`. Each names the coarse list it refines: `current`, `velocity` and `pwm`
# split `no_position` exactly; `current_position` is the current-based part of `unbounded`.
MODE_REFINED = {
    "current": "no_position",
    "velocity": "no_position",
    "pwm": "no_position",
    "current_position": "unbounded",
}
# Role rows (contract 4.3): name -> (access, area); None means any.
ROLES = {
    "Model Number": ("R", "EEPROM"),
    "Firmware Version": (None, None),
    "Torque Enable": ("RW", "RAM"),
    "Goal Position": ("RW", "RAM"),
    "Present Position": ("R", "RAM"),
}
LIMIT_FORMS = (("Min Position Limit", "Max Position Limit"), ("CW Angle Limit", "CCW Angle Limit"))
# Optional role: xtask check 14 keeps it within Protocol 1 addresses when present (X1, resolve.rs).
FAULT_ROLE = "Hardware Error Status"
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
        report.add(
            "protocols-missing", file, "", "missing `protocols`, expected [P1], [P2] or [P1, P2]", field="protocols"
        )
        return
    if not isinstance(protocols, list):
        report.add(
            "protocols-not-list",
            file,
            "`protocols`",
            f"must be a list such as [P1, P2], found {protocols!r}",
            field="protocols",
            found=protocols,
        )
        return
    if not protocols:
        found = _describe(protocols)
        report.add(
            "protocols-empty", file, "`protocols`", f"must not be empty, found {found}", field="protocols", found=found
        )
        return
    seen: list[str] = []
    for item in protocols:
        name = item.name if isinstance(item, Ident) else None
        if name not in PROTOCOLS:
            report.add(
                "protocol-unknown",
                file,
                "`protocols`",
                f"unknown protocol {item!r} (P1 or P2)",
                field="protocols",
                found=item,
                allowed=PROTOCOLS,
            )
        elif name in seen:
            report.add("protocol-duplicate", file, "`protocols`", f"{name} listed twice", field="protocols", found=name)
        else:
            seen.append(name)


ROW_FIELD_CAUSES = (
    ("access", ACCESS_VALUES, "row-access-missing", "row-access-invalid"),
    ("area", AREA_VALUES, "row-area-missing", "row-area-invalid"),
)


def check_row_fields(stem: str, table: dict, report: Report) -> None:
    """Every row has `access` (R or RW) and `area` (EEPROM or RAM) (FR-007)."""
    file = f"{stem}.ron"
    for row in table_rows(table):
        loc = _row_location(row)
        register = row["data_name"]
        for key, allowed, missing, invalid in ROW_FIELD_CAUSES:
            if key not in row:
                report.add(missing, file, loc, f"missing `{key}`", register=register, field=key)
            elif not (isinstance(row[key], Ident) and row[key].name in allowed):
                report.add(
                    invalid,
                    file,
                    loc,
                    f"`{key}` must be {' or '.join(allowed)}, found {row[key]}",
                    register=register,
                    field=key,
                    found=row[key],
                    allowed=allowed,
                )
        unknown = sorted(set(row) - {"address", "size", "data_name", "access", "area", "range"})
        if unknown:
            report.add(
                "row-unknown-field",
                file,
                loc,
                f"unknown field(s) {', '.join(unknown)}",
                register=register,
                found=unknown,
            )


def _has(row: dict, access: str | None, area: str | None) -> bool:
    return (access is None or row.get("access") == Ident(access)) and (area is None or row.get("area") == Ident(area))


def _attributes(row: dict) -> tuple[object, object]:
    """The row's (access, area) as names, None where absent."""
    return _name(row.get("access")), _name(row.get("area"))


def mode_rows(table: object) -> list[dict]:
    return [r for r in table_rows(table) if r["data_name"] in MODE_NAMES]


def check_mode_source(stem: str, table: dict, report: Report) -> None:
    """Exactly one mode source, and `operating_modes` exactly with a mode row (FR-020, research R-15)."""
    file = f"{stem}.ron"
    rows = table_rows(table)
    names = [r["data_name"] for r in rows]
    op, cm = names.count("Operating Mode"), names.count("Control Mode")
    pair = "CW Angle Limit" in names and "CCW Angle Limit" in names
    angle_limits = [n for n in ("CW Angle Limit", "CCW Angle Limit") if n in names]
    for name, count in (("Operating Mode", op), ("Control Mode", cm)):
        if count > 1:
            report.add(
                "mode-row-duplicate",
                file,
                "",
                f"{count} `{name}` rows; a table has at most one",
                register=name,
                count=count,
            )
    if op and cm:
        report.add(
            "mode-rows-both",
            file,
            "",
            "both an `Operating Mode` and a `Control Mode` row (two mode sources)",
            found=["Operating Mode", "Control Mode"],
        )
    if op and angle_limits:
        report.add(
            "mode-row-beside-angle-limits",
            file,
            "",
            f"an `Operating Mode` row beside {' and '.join(angle_limits)} rows (two mode sources)",
            register="Operating Mode",
            found=angle_limits,
        )
    if cm and stem != "xl320":
        report.add(
            "control-mode-outside-xl320", file, "", "a `Control Mode` row outside xl320.ron", register="Control Mode"
        )
    if not op and not cm and not pair:
        report.add(
            "mode-source-missing",
            file,
            "",
            "no mode source: no `Operating Mode` or `Control Mode` row and no CW/CCW Angle Limit pair",
        )
    for row in mode_rows(table):
        if row["size"] != 1 or not _has(row, "RW", "EEPROM"):
            found = (row["size"], *_attributes(row))
            report.add(
                "mode-row-attributes",
                file,
                _row_location(row),
                f"the mode register must be size 1, RW, EEPROM, found size {found[0]}, {_shown(found[1])}, {_shown(found[2])}",
                register=row["data_name"],
                found=found,
                expected=(1, "RW", "EEPROM"),
            )
    modes = table.get("operating_modes")
    mode_name = "Operating Mode" if op else "Control Mode" if cm else None
    if modes is None:
        if mode_name:
            report.add(
                "operating-modes-missing",
                file,
                "`operating_modes`",
                f"missing beside the `{mode_name}` row",
                register=mode_name,
                field="operating_modes",
            )
        return
    if not mode_name:
        report.add(
            "operating-modes-without-mode-row",
            file,
            "`operating_modes`",
            "present without an `Operating Mode` or `Control Mode` row",
            field="operating_modes",
        )
    check_operating_modes(file, modes, report, "unit_source" in table)


def _mode_values(modes: dict, key: str) -> set[int]:
    values = modes.get(key)
    if not isinstance(values, list):
        return set()
    return {v for v in values if isinstance(v, int) and not isinstance(v, bool)}


def _check_mode_list(file: str, loc: str, key: str, values: object, report: Report) -> list[int]:
    """One list of `operating_modes`: reports a non-list and each value outside 0–255; returns the valid values."""
    field = f"operating_modes.{key}"
    if not isinstance(values, list):
        report.add(
            "mode-list-not-list",
            file,
            loc,
            f"`{key}` must be a list of mode values, found {values!r}",
            field=field,
            found=values,
        )
        return []
    valid: list[int] = []
    for value in values:
        if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 255:
            report.add(
                "mode-value-invalid",
                file,
                loc,
                f"`{key}`: {value!r} is not a mode value 0–255",
                field=field,
                found=value,
                allowed=(0, 255),
            )
        else:
            valid.append(value)
    return valid


def check_operating_modes(file: str, modes: object, report: Report, has_unit_source: bool = False) -> None:
    loc = "`operating_modes`"
    if not _is_record(modes):
        found = _describe(modes)
        report.add(
            "operating-modes-not-record",
            file,
            loc,
            f"must be `(bounded: [...], unbounded: [...], no_position: [...])`, found {found}",
            field="operating_modes",
            found=found,
        )
        return
    for key in MODE_LISTS:
        if key not in modes:
            report.add("operating-modes-key-missing", file, loc, f"missing `{key}`", field=f"operating_modes.{key}")
    unknown = sorted(set(modes) - set(MODE_LISTS) - set(MODE_REFINED))
    if unknown:
        report.add(
            "operating-modes-unknown-key",
            file,
            loc,
            f"unknown key(s) {', '.join(unknown)}",
            field="operating_modes",
            found=unknown,
        )
    refined = [key for key in MODE_REFINED if key in modes]
    if refined:
        if not has_unit_source:
            report.add(
                "per-mode-lists-need-unit-source",
                file,
                loc,
                f"per-mode lists {', '.join(refined)} without `unit_source`",
                field="operating_modes",
                found=refined,
            )
        for key in MODE_REFINED:
            if key not in modes:
                report.add(
                    "per-mode-list-missing",
                    file,
                    loc,
                    f"missing `{key}` beside {', '.join(refined)}",
                    field=f"operating_modes.{key}",
                    found=refined,
                )
    for key in refined:
        _check_mode_list(file, loc, key, modes[key], report)
    if len(refined) == len(MODE_REFINED):
        split = {key: _mode_values(modes, key) for key in MODE_REFINED}
        for key, parent in MODE_REFINED.items():
            outside = sorted(split[key] - _mode_values(modes, parent))
            if outside:
                report.add(
                    "per-mode-value-outside-parent",
                    file,
                    loc,
                    f"`{key}` value(s) {outside} not in `{parent}`",
                    field=f"operating_modes.{key}",
                    found=outside,
                    parent=parent,
                )
        seen: dict[int, str] = {}
        for key in ("current", "velocity", "pwm"):
            for value in sorted(split[key]):
                if value in seen:
                    report.add(
                        "per-mode-value-twice",
                        file,
                        loc,
                        f"value {value} in both `{seen[value]}` and `{key}`",
                        field=f"operating_modes.{key}",
                        found=value,
                        lists=[seen[value], key],
                    )
                seen.setdefault(value, key)
        uncovered = sorted(_mode_values(modes, "no_position") - set(seen))
        if uncovered:
            report.add(
                "per-mode-value-uncovered",
                file,
                loc,
                f"`no_position` value(s) {uncovered} in none of `current`, `velocity`, `pwm`",
                field="operating_modes.no_position",
                found=uncovered,
            )
    where: dict[int, list[str]] = {}
    for key in MODE_LISTS:
        if key not in modes:
            continue
        for value in _check_mode_list(file, loc, key, modes[key], report):
            if key not in where.setdefault(value, []):
                where[value].append(key)
    for value, keys in sorted(where.items()):
        if len(keys) > 1:
            both = " and ".join(f"`{k}`" for k in keys)
            report.add(
                "mode-value-twice",
                file,
                loc,
                f"value {value} in both {both}",
                field="operating_modes",
                found=value,
                lists=keys,
            )


def check_roles(stem: str, table: dict, info: MapInfo, report: Report) -> None:
    """Role rules for a mapped table (contract 4.3, xtask checks 10–12, 14–16)."""
    file = f"{stem}.ron"
    rows = table_rows(table)
    by_name: dict[str, list[dict]] = {}
    for row in rows:
        by_name.setdefault(row["data_name"], []).append(row)
    for name, (access, area) in ROLES.items():
        if name not in by_name:
            report.add("role-missing", file, "", f"missing role row `{name}`", register=name)
            continue
        for row in by_name[name]:
            if not _has(row, access, area):
                found = _attributes(row)
                report.add(
                    "role-attributes",
                    file,
                    _row_location(row),
                    f"role `{name}` must be {access}/{area}, found {_shown(found[0])}/{_shown(found[1])}",
                    register=name,
                    found=found,
                    expected=(access, area),
                )
    for row in by_name.get("Model Number", []):
        if (row["address"], row["size"]) != (0, 2):
            report.add(
                "model-number-place",
                file,
                _row_location(row),
                f"`Model Number` must be at address 0, size 2, found address {row['address']}, size {row['size']}",
                register="Model Number",
                found=(row["address"], row["size"]),
                expected=(0, 2),
            )
    complete = [form for form in LIMIT_FORMS if all(n in by_name for n in form)]
    for form in LIMIT_FORMS:
        present = [n for n in form if n in by_name]
        if len(present) == 1:
            missing = next(n for n in form if n not in by_name)
            report.add(
                "limit-pair-half",
                file,
                "",
                f"half a limit pair: `{present[0]}` without `{missing}`",
                register=present[0],
                missing=missing,
            )
    if len(complete) == 2:
        shown = " and ".join(" + ".join(form) for form in complete)
        report.add(
            "limit-forms-both", file, "", f"both limit forms, {shown}", found=[n for form in complete for n in form]
        )
    elif not complete:
        report.add(
            "limit-pair-missing", file, "", "no complete limit pair (Min/Max Position Limit or CW/CCW Angle Limit)"
        )
    for form in complete:
        for name in form:
            for row in by_name[name]:
                if not _has(row, "RW", "EEPROM"):
                    found = _attributes(row)
                    report.add(
                        "limit-attributes",
                        file,
                        _row_location(row),
                        f"limit `{name}` must be RW/EEPROM, found {_shown(found[0])}/{_shown(found[1])}",
                        register=name,
                        found=found,
                        expected=("RW", "EEPROM"),
                    )
    if info.under(stem, "p1"):
        role_names = set(ROLES) | set(MODE_NAMES) | {n for form in LIMIT_FORMS for n in form} | {FAULT_ROLE}
        for row in rows:
            if row["data_name"] in role_names and row["address"] > 255:
                report.add(
                    "p1-address-above-255",
                    file,
                    _row_location(row),
                    f"role or mode row at address {row['address']}, above 255, in a table mapped under `p1`",
                    register=row["data_name"],
                    field="address",
                    found=row["address"],
                    allowed=(0, 255),
                )
    constants: dict[str, dict] = {}
    for row in rows:
        ident = const_ident(row["data_name"])
        if ident in constants:
            first = constants[ident]
            report.add(
                "register-constant-duplicate",
                file,
                _row_location(row),
                f"gives the register constant `{ident}`, like {_row_location(first)}",
                register=row["data_name"],
                found=ident,
                other=first["data_name"],
            )
        else:
            constants[ident] = row
    check_type_info(file, table.get("type_info"), report)


def check_type_info(file: str, scale: object, report: Report) -> None:
    """Scale sanity (xtask check 15): max > min for values and radians, zero within min..=max."""
    loc = "`type_info`"
    if scale is None:
        report.add("type-info-missing", file, loc, "missing (the position scale)", field="type_info")
        return
    if not _is_record(scale):
        found = _describe(scale)
        report.add(
            "type-info-not-record", file, loc, f"must be a record, found {found}", field="type_info", found=found
        )
        return
    missing = [k for k in TYPE_INFO_KEYS if k not in scale]
    if missing:
        report.add(
            "type-info-key-missing", file, loc, f"missing {', '.join(missing)}", field="type_info", missing=missing
        )
        return
    zero, lo, hi = (scale[k] for k in TYPE_INFO_KEYS[:3])
    rmin, rmax = scale["min_radian"], scale["max_radian"]
    if not all(isinstance(v, int) and not isinstance(v, bool) for v in (zero, lo, hi)):
        report.add(
            "type-info-position-not-int",
            file,
            loc,
            f"the zero, min and max position values must be integers, found {zero!r}, {lo!r}, {hi!r}",
            field="type_info",
            found=(zero, lo, hi),
        )
        return
    if not all(isinstance(v, (int, float)) and not isinstance(v, bool) for v in (rmin, rmax)):
        report.add(
            "type-info-radian-not-number",
            file,
            loc,
            f"`min_radian` and `max_radian` must be numbers, found {rmin!r}, {rmax!r}",
            field="type_info",
            found=(rmin, rmax),
        )
        return
    if hi <= lo:
        report.add(
            "type-info-max-not-above-min",
            file,
            loc,
            f"value_of_max_radian_position ({hi}) must be above value_of_min_radian_position ({lo})",
            field="type_info",
            max=hi,
            min=lo,
        )
    if rmax <= rmin:
        report.add(
            "type-info-max-radian-not-above-min",
            file,
            loc,
            f"max_radian ({rmax}) must be above min_radian ({rmin})",
            field="type_info",
            max=rmax,
            min=rmin,
        )
    if not lo <= zero <= hi:
        report.add(
            "type-info-zero-outside",
            file,
            loc,
            f"value_of_zero_radian_position ({zero}) must lie within {lo}..={hi}",
            field="type_info",
            found=zero,
            allowed=(lo, hi),
        )


# --------------------------------------------------------------------------
# FR-9 register data (specs/009-tables-fr9/contracts/fr9-schema.md)
# --------------------------------------------------------------------------
#
# `unit_source: Verified(url: …, verified: …)` (optional, any table) records the
# e-Manual page and date on which the table's FR-9 data was checked: its
# `unit_info`, row `range`s, per-mode `operating_modes` lists and
# `torque_control_mode`. A table without it keeps the legacy `unit_info` shape
# (the four keys below, sign type `signed` or `unsigned`, any unit) and may not
# carry the other FR-9 fields.
#
# A `unit_info` entry under `unit_source` is
#
#     { data_name: "Moving Speed", value: 0.111, unit: "rpm", sign_type: "unsigned",
#       modes: ["joint"], zero_is_unlimited: true, range: (min: 0, max: 1023) }
#
# - `data_name`: the exact `data_name` of a row of this table.
# - `value`: the e-Manual's size of one raw step, in `unit`; above zero.
# - `unit`: one of UNITS, as the e-Manual writes it (`rev/min` is `rpm`,
#   `1 [°C]` is `°C`, `0.1 [V]` is `V`, `0.1%` is `%`, `1 [msec]` is `ms`).
# - `sign_type`: `unsigned` (raw is a magnitude), `twos_complement` (Protocol 2
#   pages: "Two's complement is applied for the negative value") or
#   `direction_bit_N` (Protocol 1 pages: "the 10th bit becomes the direction
#   bit"; bits below N are the magnitude, bit N set means CW, read as negative).
# - `modes` (optional): the angle-limit modes of an AX / RX / MX Protocol 1
#   table the entry applies in, from MODES_P1 (`torque_control` needs
#   `torque_control_mode`); absent means every mode.
# - `profile` (optional): the Drive Mode profile the entry applies in, from
#   PROFILES; only in a table with a `Drive Mode` row; absent means both.
# - No two entries of one register meet in a mode and profile; an entry without
#   `modes` (or `profile`) meets every other entry of that register.
# - `zero_is_unlimited` (optional): `true` where the page says raw 0 means no
#   limit ("maximum rpm", "infinite velocity").
# - `range` (optional): `(min: INT, max: INT)`, the raw range the page gives in
#   this mode; it fits the row size under `sign_type`.
# - `bounded_by` (optional): the row whose value bounds this one's magnitude
#   (`Goal Current` by `Current Limit`); that row has an entry of the same
#   `value` and `unit`.
# - `from_fw` (optional): the first firmware version the register works on.
#
# A row may carry `range: (min: INT, max: INT)` (raw, from the page's Range
# column or description) under `unit_source`; it fits the row size.
#
# `torque_control_mode: (register: STRING, on: INT, goal: STRING)` (MX on
# Protocol 1): the RW RAM size-1 row that switches torque control on when it
# holds `on` (inside that row's `range`, when it has one), and a different RW
# RAM row that is then the goal; only in a table whose mode source is the
# CW/CCW Angle Limit pair.

UNIT_INFO_KEYS = ("data_name", "value", "unit", "sign_type")
UNIT_INFO_FR9_KEYS = ("modes", "profile", "zero_is_unlimited", "range", "bounded_by", "from_fw")
UNITS = ("deg", "rpm", "mA", "V", "°C", "%", "ms", "deg/s^2")
SIGN_TYPES = ("unsigned", "twos_complement")
LEGACY_SIGN_TYPES = ("signed", "unsigned")
DIRECTION_BIT_RE = re.compile(r"^direction_bit_(\d+)$")
MODES_P1 = ("joint", "wheel", "multi_turn", "torque_control")
PROFILES = ("velocity_based", "time_based")
TORQUE_CONTROL_KEYS = ("register", "on", "goal")


def _is_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _is_number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def raw_bounds(size: int, sign_type: str | None) -> tuple[int, int]:
    """The raw values a register of `size` bytes holds under `sign_type` (`None`: either reading)."""
    bits = 8 * size
    match = DIRECTION_BIT_RE.match(sign_type or "")
    if match:
        return 0, (1 << (int(match.group(1)) + 1)) - 1
    if sign_type == "twos_complement":
        return -(1 << (bits - 1)), (1 << (bits - 1)) - 1
    if sign_type == "unsigned":
        return 0, (1 << bits) - 1
    return -(1 << (bits - 1)), (1 << bits) - 1


def check_range(
    file: str, loc: str, value: object, size: int, sign_type: str | None, report: Report, **context: object
) -> None:
    """`(min: INT, max: INT)`, `min <= max`, both raw values of the register."""
    if not _is_record(value) or set(value) != {"min", "max"}:
        report.add(
            "range-shape", file, loc, f"`range` must be `(min: INT, max: INT)`, found {value!r}", found=value, **context
        )
        return
    lo, hi = value["min"], value["max"]
    if not (_is_int(lo) and _is_int(hi)):
        report.add(
            "range-not-int",
            file,
            loc,
            f"`range` min and max must be integers, found {lo!r} and {hi!r}",
            found=(lo, hi),
            **context,
        )
        return
    if lo > hi:
        report.add("range-min-above-max", file, loc, f"`range` min {lo} is above max {hi}", min=lo, max=hi, **context)
    first, last = raw_bounds(size, sign_type)
    if sign_type is None and lo < 0:
        last = (1 << (8 * size - 1)) - 1
    if lo < first or hi > last:
        shown = f" as {sign_type}" if sign_type else ""
        report.add(
            "range-does-not-fit",
            file,
            loc,
            f"`range` {lo}..={hi} does not fit a {size}-byte register{shown} ({first}..={last})",
            found=(lo, hi),
            allowed=(first, last),
            size=size,
            sign_type=sign_type,
            **context,
        )


def mode_source_is_angle_limits(table: dict) -> bool:
    names = {r["data_name"] for r in table_rows(table)}
    return not (names & set(MODE_NAMES)) and {"CW Angle Limit", "CCW Angle Limit"} <= names


def check_unit_source(stem: str, table: dict, today: datetime.date, report: Report) -> bool:
    """`unit_source`, when present, is `Verified(url, verified)`; returns whether it is present."""
    if "unit_source" not in table:
        return False
    check_verified(f"{stem}.ron", "unit_source", table["unit_source"], today, report, "`Verified(url: …, verified: …)`")
    return True


def check_unit_info(stem: str, table: dict, strict: bool, report: Report) -> None:
    """`unit_info` entries: the legacy shape, or the FR-9 shape under `unit_source` (`strict`)."""
    file = f"{stem}.ron"
    entries = table.get("unit_info")
    if entries is None:
        return
    if not isinstance(entries, list):
        found = _describe(entries)
        report.add(
            "unit-info-not-list",
            file,
            "`unit_info`",
            f"must be a list of entries, found {found}",
            field="unit_info",
            found=found,
        )
        return
    rows = {r["data_name"]: r for r in table_rows(table)}
    has_drive_mode = "Drive Mode" in rows
    angle_limits = mode_source_is_angle_limits(table)
    has_torque_mode = "torque_control_mode" in table
    seen: list[tuple[str, str | None, str | None, int]] = []
    valid: list[tuple[int, dict]] = []
    for i, item in enumerate(entries):
        loc = f"`unit_info` entry {i}"
        if not _is_record(item):
            found = _describe(item)
            report.add(
                "unit-entry-not-record",
                file,
                loc,
                f"must be a record `{{ data_name: …, value: …, unit: …, sign_type: … }}`, found {found}",
                field="unit_info",
                index=i,
                found=found,
            )
            continue
        name = item.get("data_name")
        if isinstance(name, str) and name:
            loc = f'`unit_info` entry {i} "{name}"'
        else:
            report.add(
                "unit-entry-name-invalid",
                file,
                loc,
                f"`data_name` must be a non-empty string, found {name!r}",
                field="data_name",
                index=i,
                found=name,
            )
        context: dict = {"register": name if isinstance(name, str) and name else None, "index": i}
        for key in UNIT_INFO_KEYS[1:]:
            if key not in item:
                report.add("unit-entry-key-missing", file, loc, f"missing `{key}`", field=key, **context)
        value, unit, sign = item.get("value"), item.get("unit"), item.get("sign_type")
        if "value" in item and not (_is_number(value) and value > 0):
            report.add(
                "unit-entry-value-invalid",
                file,
                loc,
                f"`value` must be a number above 0, found {value!r}",
                field="value",
                found=value,
                **context,
            )
        if "unit" in item and not (isinstance(unit, str) and unit):
            report.add(
                "unit-entry-unit-invalid",
                file,
                loc,
                f"`unit` must be a non-empty string, found {unit!r}",
                field="unit",
                found=unit,
                **context,
            )
        unknown = sorted(set(item) - set(UNIT_INFO_KEYS) - set(UNIT_INFO_FR9_KEYS))
        if not strict:
            fr9 = sorted(set(item) & set(UNIT_INFO_FR9_KEYS))
            if fr9:
                report.add(
                    "unit-entry-fr9-needs-unit-source",
                    file,
                    loc,
                    f"{', '.join(fr9)} without `unit_source`",
                    found=fr9,
                    **context,
                )
            if unknown:
                report.add(
                    "unit-entry-unknown-field",
                    file,
                    loc,
                    f"unknown field(s) {', '.join(unknown)}",
                    found=unknown,
                    **context,
                )
            if "sign_type" in item and sign not in LEGACY_SIGN_TYPES and not _is_fr9_sign(sign):
                report.add(
                    "unit-entry-sign-unknown",
                    file,
                    loc,
                    f"unknown `sign_type` {sign!r}",
                    field="sign_type",
                    found=sign,
                    allowed=(*LEGACY_SIGN_TYPES, *SIGN_TYPES, "direction_bit_N"),
                    **context,
                )
            continue
        if unknown:
            report.add(
                "unit-entry-unknown-field",
                file,
                loc,
                f"unknown field(s) {', '.join(unknown)}",
                found=unknown,
                **context,
            )
        row = rows.get(name) if isinstance(name, str) else None
        if isinstance(name, str) and name and row is None:
            report.add("unit-entry-no-row", file, loc, "names no row of this table", **context)
        if isinstance(unit, str) and unit not in UNITS:
            report.add(
                "unit-entry-unit-unknown",
                file,
                loc,
                f"`unit` {unit!r} is not one of {', '.join(UNITS)}",
                field="unit",
                found=unit,
                allowed=UNITS,
                **context,
            )
        if "sign_type" in item:
            if not _is_fr9_sign(sign):
                report.add(
                    "unit-entry-sign-invalid",
                    file,
                    loc,
                    f"`sign_type` must be unsigned, twos_complement or direction_bit_N, found {sign!r}",
                    field="sign_type",
                    found=sign,
                    **context,
                )
                sign = None
            elif row is not None:
                match = DIRECTION_BIT_RE.match(sign)
                last_bit = 8 * row["size"] - 1
                if match and not 1 <= int(match.group(1)) <= last_bit:
                    bit = int(match.group(1))
                    report.add(
                        "unit-entry-direction-bit-outside",
                        file,
                        loc,
                        f"`{sign}`: bit {bit} is outside a {row['size']}-byte register (1..={last_bit})",
                        field="sign_type",
                        found=bit,
                        size=row["size"],
                        allowed=(1, last_bit),
                        **context,
                    )
                    sign = None
        else:
            sign = None
        modes = item.get("modes")
        mode_keys: list[str | None] = [None]
        if "modes" in item:
            if not angle_limits:
                report.add(
                    "unit-entry-modes-need-angle-limits",
                    file,
                    loc,
                    "`modes` in a table whose mode source is not the CW/CCW Angle Limit pair",
                    field="modes",
                    **context,
                )
            if not isinstance(modes, list) or not modes or not all(isinstance(m, str) for m in modes):
                report.add(
                    "unit-entry-modes-invalid",
                    file,
                    loc,
                    f"`modes` must be a non-empty list of {', '.join(MODES_P1)}, found {modes!r}",
                    field="modes",
                    found=modes,
                    **context,
                )
            else:
                mode_keys = []
                for mode in modes:
                    if mode not in MODES_P1:
                        report.add(
                            "unit-entry-mode-unknown",
                            file,
                            loc,
                            f"unknown mode {mode!r} (one of {', '.join(MODES_P1)})",
                            field="modes",
                            found=mode,
                            allowed=MODES_P1,
                            **context,
                        )
                    elif mode == "torque_control" and not has_torque_mode:
                        report.add(
                            "unit-entry-mode-needs-torque-control",
                            file,
                            loc,
                            "mode `torque_control` without `torque_control_mode`",
                            field="modes",
                            found=mode,
                            **context,
                        )
                    if mode in mode_keys:
                        report.add(
                            "unit-entry-mode-duplicate",
                            file,
                            loc,
                            f"mode {mode!r} listed twice",
                            field="modes",
                            found=mode,
                            **context,
                        )
                    else:
                        mode_keys.append(mode)
        profile = item.get("profile")
        if "profile" in item:
            if profile not in PROFILES:
                report.add(
                    "unit-entry-profile-invalid",
                    file,
                    loc,
                    f"`profile` must be one of {', '.join(PROFILES)}, found {profile!r}",
                    field="profile",
                    found=profile,
                    allowed=PROFILES,
                    **context,
                )
            if not has_drive_mode:
                report.add(
                    "unit-entry-profile-needs-drive-mode",
                    file,
                    loc,
                    "`profile` in a table without a `Drive Mode` row",
                    field="profile",
                    **context,
                )
        if "zero_is_unlimited" in item and item["zero_is_unlimited"] not in (Ident("true"), Ident("false")):
            report.add(
                "unit-entry-zero-is-unlimited-invalid",
                file,
                loc,
                f"`zero_is_unlimited` must be true or false, found {item['zero_is_unlimited']!r}",
                field="zero_is_unlimited",
                found=item["zero_is_unlimited"],
                **context,
            )
        if "range" in item and row is not None:
            check_range(file, loc, item["range"], row["size"], sign, report, field="range", **context)
        from_fw = item.get("from_fw")
        if "from_fw" in item and not (_is_int(from_fw) and 0 <= from_fw <= 255):
            report.add(
                "unit-entry-from-fw-invalid",
                file,
                loc,
                f"`from_fw` must be an integer 0–255, found {from_fw!r}",
                field="from_fw",
                found=from_fw,
                allowed=(0, 255),
                **context,
            )
        if isinstance(name, str):
            selector = profile if isinstance(profile, str) else None
            for mode in mode_keys:
                clash = next(
                    (j for n, m, p, j in seen if n == name and _overlaps(m, mode) and _overlaps(p, selector)), None
                )
                if clash is not None:
                    detail = (f", mode {mode}" if mode else "") + (f", profile {selector}" if selector else "")
                    report.add(
                        "unit-entry-overlap",
                        file,
                        loc,
                        f"same register, mode and profile as `unit_info` entry {clash}{detail}",
                        other=clash,
                        mode=mode,
                        profile=selector,
                        **context,
                    )
                else:
                    seen.append((name, mode, selector, i))
            valid.append((i, item))
    for i, item in valid:
        bound = item.get("bounded_by")
        if bound is None:
            continue
        loc = f'`unit_info` entry {i} "{item["data_name"]}"'
        context = {"register": item["data_name"] or None, "field": "bounded_by", "index": i}
        if not isinstance(bound, str) or bound not in rows:
            report.add(
                "bounded-by-no-row",
                file,
                loc,
                f"`bounded_by` {bound!r} names no row of this table",
                found=bound,
                **context,
            )
            continue
        if bound == item["data_name"]:
            report.add(
                "bounded-by-self",
                file,
                loc,
                f"`bounded_by` {bound!r} names the register itself",
                found=bound,
                **context,
            )
            continue
        scale = (item.get("value"), item.get("unit"))
        if not any(e["data_name"] == bound and (e.get("value"), e.get("unit")) == scale for _j, e in valid):
            report.add(
                "bounded-by-scale-mismatch",
                file,
                loc,
                f"`bounded_by` {bound!r} has no `unit_info` entry of value {scale[0]!r} and unit {scale[1]!r}",
                found=bound,
                expected=scale,
                **context,
            )


def _overlaps(a: str | None, b: str | None) -> bool:
    """Two `modes` / `profile` selectors meet: equal, or one is absent (absent means every one)."""
    return a is None or b is None or a == b


def _is_fr9_sign(sign: object) -> bool:
    return isinstance(sign, str) and (sign in SIGN_TYPES or DIRECTION_BIT_RE.match(sign) is not None)


def check_row_ranges(stem: str, table: dict, strict: bool, report: Report) -> None:
    """Row `range`s: only under `unit_source`, each fitting its row."""
    file = f"{stem}.ron"
    for row in table_rows(table):
        if "range" not in row:
            continue
        loc = _row_location(row)
        if not strict:
            report.add(
                "row-range-needs-unit-source",
                file,
                loc,
                "`range` without `unit_source`",
                register=row["data_name"],
                field="range",
            )
            continue
        check_range(file, loc, row["range"], row["size"], None, report, register=row["data_name"], field="range")


def check_torque_control_mode(stem: str, table: dict, strict: bool, info: MapInfo, report: Report) -> None:
    """`torque_control_mode`: an RW RAM switch row and an RW RAM goal row, angle-limit tables only."""
    file = f"{stem}.ron"
    loc = "`torque_control_mode`"
    spec = table.get("torque_control_mode")
    if spec is None:
        return
    if not strict:
        report.add(
            "torque-control-needs-unit-source", file, loc, "present without `unit_source`", field="torque_control_mode"
        )
    if not mode_source_is_angle_limits(table):
        report.add(
            "torque-control-needs-angle-limits",
            file,
            loc,
            "present in a table whose mode source is not the CW/CCW Angle Limit pair",
            field="torque_control_mode",
        )
    if not _is_record(spec):
        found = _describe(spec)
        report.add(
            "torque-control-not-record",
            file,
            loc,
            f"must be `(register: STRING, on: INT, goal: STRING)`, found {found}",
            field="torque_control_mode",
            found=found,
        )
        return
    for key in TORQUE_CONTROL_KEYS:
        if key not in spec:
            report.add("torque-control-key-missing", file, loc, f"missing `{key}`", field=f"torque_control_mode.{key}")
    unknown = sorted(set(spec) - set(TORQUE_CONTROL_KEYS))
    if unknown:
        report.add(
            "torque-control-unknown-key",
            file,
            loc,
            f"unknown field(s) {', '.join(unknown)}",
            field="torque_control_mode",
            found=unknown,
        )
    on = spec.get("on")
    if "on" in spec and not (_is_int(on) and 0 <= on <= 255):
        report.add(
            "torque-control-on-invalid",
            file,
            loc,
            f"`on` must be a register value 0–255, found {on!r}",
            field="torque_control_mode.on",
            found=on,
            allowed=(0, 255),
        )
    rows = {r["data_name"]: r for r in table_rows(table)}
    for key, size in (("register", 1), ("goal", None)):
        name = spec.get(key)
        if key not in spec:
            continue
        field = f"torque_control_mode.{key}"
        row = rows.get(name) if isinstance(name, str) else None
        if row is None:
            report.add(
                "torque-control-no-row",
                file,
                loc,
                f"`{key}` {name!r} names no row of this table",
                field=field,
                found=name,
            )
            continue
        if not _has(row, "RW", "RAM") or (size is not None and row["size"] != size):
            want = f"size {size}, RW, RAM" if size else "RW, RAM"
            found = (row["size"], *_attributes(row))
            report.add(
                "torque-control-row-attributes",
                file,
                loc,
                f"`{key}` row must be {want}; {_row_location(row)} is size {found[0]}, {_shown(found[1])}, {_shown(found[2])}",
                register=row["data_name"],
                field=field,
                found=found,
                expected=(size, "RW", "RAM"),
            )
        if info.under(stem, "p1") and row["address"] > 255:
            # DECISION: the same cause as a role row above 255 (one rule, xtask check 14); `field` tells them apart.
            report.add(
                "p1-address-above-255",
                file,
                loc,
                f"`{key}` row at address {row['address']}, above 255, in a table mapped under `p1`",
                register=row["data_name"],
                field=field,
                found=row["address"],
                allowed=(0, 255),
            )
        span = row.get("range")
        bounded = _is_record(span) and _is_int(span.get("min")) and _is_int(span.get("max"))
        if key == "register" and _is_int(on) and bounded:
            if not span["min"] <= on <= span["max"]:
                report.add(
                    "torque-control-on-outside-range",
                    file,
                    loc,
                    f"`on` {on} is outside the `register` row range {span['min']}..={span['max']}",
                    register=row["data_name"],
                    field="torque_control_mode.on",
                    found=on,
                    allowed=(span["min"], span["max"]),
                )
    if "register" in spec and spec.get("register") == spec.get("goal"):
        name = spec.get("register")
        report.add(
            "torque-control-same-row",
            file,
            loc,
            f"`register` and `goal` both name {name!r}",
            register=name if isinstance(name, str) else None,
            field="torque_control_mode",
        )


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
                shown = [k for k in ("p1", "p2") if k in keys]
                parts.append(" ".join([str(number), *shown]))
            return ", ".join(parts)
        if stem in self.not_mapped:
            return f"not mapped: {self.not_mapped[stem]}"
        return "not mapped"


LIST_KEYS = ("p1", "p2")


def read_map(model_map: object, report: Report) -> MapInfo:
    """Collects which file sits under which model number and list; reports shape errors."""
    info = MapInfo()
    if not _is_record(model_map):
        found = _describe(model_map)
        report.add(
            "map-not-record", MAP_FILE, "", f"the map is not a record `( models: {{ … }} )`, found {found}", found=found
        )
        return info
    models = model_map.get("models")
    if not isinstance(models, dict) or any(not isinstance(k, int) for k in models):
        found = _describe(models)
        report.add(
            "models-not-map",
            MAP_FILE,
            "`models`",
            f"must be a map `{{ NUMBER: ( … ) }}`, found {found}",
            field="models",
            found=found,
        )
        return info
    for number, entry in models.items():
        if not 0 <= number <= 0xFFFF:
            report.add(
                "model-number-invalid",
                MAP_FILE,
                f"model {number}",
                f"model number must be 0–65535, found {number}",
                model=number,
                found=number,
                allowed=(0, 0xFFFF),
            )
        if not _is_record(entry):
            found = _describe(entry)
            report.add(
                "model-entry-not-record",
                MAP_FILE,
                f"model {number}",
                f"entry must be a record `( … )`, found {found}",
                model=number,
                found=found,
            )
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


def _refs(entry: dict, key: str, number: int, loc: str, report: Report) -> list[tuple[int, str]] | None:
    """A `p1`/`p2` list as (from_fw, file) pairs, or None when malformed."""
    refs = entry.get(key)
    if not isinstance(refs, list):
        report.add(
            "model-list-not-list",
            MAP_FILE,
            loc,
            f"`{key}` must be a list of `(from_fw: …, file: …)`, found {refs!r}",
            model=number,
            field=key,
            found=refs,
        )
        return None
    out = []
    for ref in refs:
        if not (_is_record(ref) and isinstance(ref.get("from_fw"), int) and isinstance(ref.get("file"), str)):
            report.add(
                "model-ref-invalid",
                MAP_FILE,
                loc,
                f"`{key}`: {ref!r} is not `(from_fw: INT, file: STRING)`",
                model=number,
                field=key,
                found=ref,
            )
            return None
        out.append((ref["from_fw"], ref["file"]))
    return out


def check_map(tables: dict[str, object], info: MapInfo, report: Report) -> None:
    """Model map rules (data-model "Model entry", "Table ref", "Map validation"; ron-schema §2)."""
    for number, entry in sorted(info.models.items()):
        loc = f"model {number}"
        family = entry.get("family")
        if family not in FAMILIES:
            report.add(
                "model-family-invalid",
                MAP_FILE,
                loc,
                f"`family` must be one of {', '.join(FAMILIES)}, found {family!r}",
                model=number,
                field="family",
                found=family,
                allowed=FAMILIES,
            )
        name = entry.get("name")
        if not isinstance(name, str) or not name:
            report.add(
                "model-name-invalid",
                MAP_FILE,
                loc,
                f"`name` must be a non-empty string, found {name!r}",
                model=number,
                field="name",
                found=name,
            )
        if "also" in entry:
            also = entry["also"]
            if not isinstance(also, list) or not also:
                report.add(
                    "model-also-shape",
                    MAP_FILE,
                    loc,
                    f"`also` must be a non-empty list of product names, found {also!r}",
                    model=number,
                    field="also",
                    found=also,
                )
            elif not all(isinstance(n, str) and n for n in also) or len(set(also)) != len(also):
                report.add(
                    "model-also-names",
                    MAP_FILE,
                    loc,
                    f"`also` must list distinct non-empty product names, found {also!r}",
                    model=number,
                    field="also",
                    found=also,
                )
        lists: dict[str, list[tuple[int, str]]] = {}
        for key in LIST_KEYS:
            if key in entry:
                refs = _refs(entry, key, number, loc, report)
                if refs is not None:
                    lists[key] = refs
        if "p1" not in entry and "p2" not in entry:
            report.add("model-lists-missing", MAP_FILE, loc, "no `p1` or `p2` list", model=number)
        for key, refs in lists.items():
            protocol = key.upper()
            if not refs:
                report.add("model-list-empty", MAP_FILE, loc, f"`{key}` is empty", model=number, field=key)
                continue
            if refs[0][0] != 0:
                report.add(
                    "model-first-from-fw",
                    MAP_FILE,
                    loc,
                    f"`{key}`: the first `from_fw` must be 0, found {refs[0][0]}",
                    model=number,
                    field=key,
                    found=refs[0][0],
                    expected=0,
                )
            outside = [fw for fw, _f in refs if not 0 <= fw <= 255]
            if outside:
                report.add(
                    "model-from-fw-range",
                    MAP_FILE,
                    loc,
                    f"`{key}`: `from_fw` must be 0–255, found {', '.join(map(str, outside))}",
                    model=number,
                    field=key,
                    found=outside,
                    allowed=(0, 255),
                )
            if any(a[0] >= b[0] for a, b in zip(refs, refs[1:])):
                order = [fw for fw, _f in refs]
                report.add(
                    "model-from-fw-order",
                    MAP_FILE,
                    loc,
                    f"`{key}`: `from_fw` must strictly increase, found {', '.join(map(str, order))}",
                    model=number,
                    field=key,
                    found=order,
                )
            for _fw, stem in refs:
                if stem not in tables:
                    report.add(
                        "model-file-missing",
                        MAP_FILE,
                        loc,
                        f"`{key}` names {stem}.ron, which does not exist",
                        model=number,
                        field=key,
                        table=stem,
                    )
                    continue
                names = _protocol_names(tables[stem]) if _is_record(tables[stem]) else None
                if names is not None and protocol not in names:
                    report.add(
                        "model-file-protocol",
                        MAP_FILE,
                        loc,
                        f"`{key}` names {stem}.ron, whose `protocols` [{', '.join(names)}] lack {protocol}",
                        model=number,
                        field=key,
                        table=stem,
                        found=names,
                        expected=protocol,
                    )
            places = set()
            for _fw, stem in refs:
                table = tables.get(stem)
                for row in table_rows(table):
                    if row["data_name"] == "Firmware Version":
                        places.add((row["address"], row["size"]))
            if len(places) > 1:
                shown = ", ".join(f"({a}, {s})" for a, s in sorted(places))
                report.add(
                    "model-firmware-version-places",
                    MAP_FILE,
                    loc,
                    f"`{key}`: `Firmware Version` sits at different places: {shown}",
                    model=number,
                    register="Firmware Version",
                    field=key,
                    found=sorted(places),
                )
        both = "p1" in entry and "p2" in entry
        default = entry.get("default")
        if both and default is None:
            report.add(
                "model-default-missing",
                MAP_FILE,
                loc,
                "no `default` while both `p1` and `p2` exist",
                model=number,
                field="default",
            )
        elif both and default != Ident("P2"):
            report.add(
                "model-default-invalid",
                MAP_FILE,
                loc,
                f"`default` must be P2, found {_shown(default)}",
                model=number,
                field="default",
                found=default,
                expected="P2",
            )
        elif not both and default is not None:
            report.add(
                "model-default-unexpected",
                MAP_FILE,
                loc,
                f"`default` {_shown(default)} with only one of `p1` and `p2`",
                model=number,
                field="default",
                found=default,
            )
        listed = {stem for refs in lists.values() for _fw, stem in refs}
        for stem in sorted(listed):
            names = _protocol_names(tables[stem]) if _is_record(tables.get(stem)) else None
            if names and "P1" in names and "P2" in names and not both:
                report.add(
                    "model-needs-both-lists",
                    MAP_FILE,
                    loc,
                    f"{stem}.ron lists both protocols, so the model needs both `p1` and `p2` lists",
                    model=number,
                    table=stem,
                )
        if "tables" in entry:
            report.add("model-legacy-tables", MAP_FILE, loc, "legacy `tables` key", model=number, field="tables")
    for stem, places in sorted(info.refs.items()):
        numbers = sorted({n for n, _k in places})
        if len(numbers) > 1:
            report.add(
                "file-under-two-numbers",
                MAP_FILE,
                "",
                f"{stem}.ron is mapped under model numbers {' and '.join(map(str, numbers))}; a file sits under one number",
                table=stem,
                found=numbers,
            )
    seen_types: dict[str, str] = {}
    for stem in sorted(info.refs):
        ident = type_ident(stem)
        if ident in seen_types:
            report.add(
                "type-ident-duplicate",
                MAP_FILE,
                "",
                f"{seen_types[ident]}.ron and {stem}.ron both give the type `{ident}`",
                table=stem,
                other=seen_types[ident],
                found=ident,
            )
        else:
            seen_types[ident] = stem
        module = module_ident(stem)
        if module == "tests" or module in RUST_KEYWORDS:
            report.add(
                "module-ident-reserved",
                MAP_FILE,
                "",
                f"{stem}.ron gives the module name `{module}`, which is `tests` or a Rust keyword",
                table=stem,
                found=module,
            )


def check_not_mapped(model_map: object, tables: dict[str, object], info: MapInfo, report: Report) -> None:
    """Every table is mapped or in `not_mapped`, never both, never neither (FR-011, xtask checks 6–7)."""
    records = model_map.get("not_mapped", []) if _is_record(model_map) else []
    if not isinstance(records, list):
        found = _describe(records)
        report.add(
            "not-mapped-not-list",
            MAP_FILE,
            "`not_mapped`",
            f"must be a list of `(file: …, reason: …)`, found {found}",
            field="not_mapped",
            found=found,
        )
        records = []
    for i, record in enumerate(records):
        loc = f"`not_mapped` item {i}"
        if not (_is_record(record) and isinstance(record.get("file"), str)):
            found = _describe(record)
            report.add(
                "not-mapped-record-invalid",
                MAP_FILE,
                loc,
                f"must be `(file: STRING, reason: NoModelNumber | SameNumberAs(N))`, found {found}",
                field="not_mapped",
                index=i,
                found=found,
            )
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
                report.add(
                    "not-mapped-number-unknown",
                    MAP_FILE,
                    loc,
                    f"SameNumberAs({number}) names a number that is not in `models`",
                    model=number,
                    table=stem,
                )
        else:
            report.add(
                "not-mapped-reason-invalid",
                MAP_FILE,
                loc,
                f"reason must be NoModelNumber or SameNumberAs(N), found {reason!r}",
                field="reason",
                table=stem,
                found=reason,
            )
            continue
        if stem not in tables:
            report.add("not-mapped-file-missing", MAP_FILE, loc, f"{stem}.ron does not exist", table=stem)
        elif info.mapped(stem):
            numbers = sorted({n for n, _k in info.refs[stem]})
            report.add(
                "not-mapped-also-mapped",
                MAP_FILE,
                loc,
                f"{stem}.ron is mapped (model {', '.join(map(str, numbers))}) and also in `not_mapped`",
                table=stem,
                models=numbers,
            )
        if stem in info.not_mapped:
            report.add("not-mapped-duplicate", MAP_FILE, loc, f"{stem} is listed twice in `not_mapped`", table=stem)
        info.not_mapped.setdefault(stem, text)
    for stem in tables:
        if not info.mapped(stem) and stem not in info.not_mapped:
            report.add(
                "table-unlisted", f"{stem}.ron", "", "not mapped and not in `not_mapped` of dynamixel.ron", table=stem
            )


def run_checks(model_map: object, tables: dict[str, object], today: datetime.date) -> tuple[Report, MapInfo]:
    """Runs every rule; returns the diagnostics and the map summary."""
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
        strict = check_unit_source(stem, table, today, report)
        check_unit_info(stem, table, strict, report)
        check_row_ranges(stem, table, strict, report)
        check_torque_control_mode(stem, table, strict, info, report)
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
        f"{len(report.diagnostics)} errors"
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
        prog=PROG,
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


class Outcome(NamedTuple):
    """One run: its exit code, what it prints on stdout and stderr, and its diagnostics."""

    code: int
    out: str
    err: str
    diagnostics: list[Diagnostic]


def run(argv: list[str] | None = None) -> Outcome:
    """Runs the check on `argv` without printing."""
    # DECISION: `run` returns what `main` prints plus the diagnostics as data, so tests check the cause code
    # and fields of each finding (NFR-12) rather than parsing the printed text.
    parser = build_parser()
    try:
        args = parser.parse_args(argv)
    except SystemExit as exc:
        return Outcome(int(exc.code or 0), "", "", [])
    if args.today is None:
        args.today = datetime.date.today()
    try:
        model_map, tables = load(args.dir)
    except (LoadError, RonError, OSError) as err:
        return Outcome(EXIT_LOAD, "", f"error: {err}\n", [])
    unknown = [stem for stem in args.stems if stem not in tables]
    if unknown:
        return Outcome(EXIT_LOAD, "", f"error: no table named {', '.join(unknown)} in {args.dir}\n", [])
    report, info = run_checks(model_map, tables, args.today)
    selected = args.stems or list(tables)
    err = "".join(f"{line}\n" for line in report.lines())
    return Outcome(report.exit_code(), render(tables, selected, report, info, args.markdown), err, report.diagnostics)


def main(argv: list[str] | None = None) -> int:
    outcome = run(argv)
    sys.stdout.write(outcome.out)
    sys.stderr.write(outcome.err)
    return outcome.code


if __name__ == "__main__":
    sys.exit(main())

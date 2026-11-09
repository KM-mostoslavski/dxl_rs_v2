"""Tests for `check_tables.py` (specs/005-tables/contracts/table-tools.md section 1, SRS FR-6.14).

Run with `python3 -m unittest discover -s control_tables -p 'test_*.py'`. No network.

A failing case checks the diagnostic's cause code and its fields (file,
location, model, register, field and the deciding values), not only its text
(NFR-12).
"""

from __future__ import annotations

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_tables  # noqa: E402
from check_tables import Ident, RonError, Tagged, parse_ron  # noqa: E402

TODAY = "2026-10-28"


def write_fixture(root: Path, files: dict[str, str]) -> Path:
    """Writes `files` (stem or file name -> text) into `root` and returns it.

    A key without an extension gets `.ron`. `dynamixel.ron` is written only
    when the caller passes it.
    """
    for name, text in files.items():
        path = root / (name if "." in name else f"{name}.ron")
        path.write_text(text, encoding="utf-8")
    return root


def run_check(argv: list[str]) -> check_tables.Outcome:
    """Runs the check on `argv`: (exit code, stdout, stderr, diagnostics)."""
    return check_tables.run(argv)


def run_main(argv: list[str]) -> tuple[int, str, str]:
    """Calls `check_tables.main(argv)` and returns (exit code, printed stdout, printed stderr)."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = check_tables.main(argv)
    return code, out.getvalue(), err.getvalue()


def _scalars(value: object) -> list[object]:
    """The ints and strings of a deciding value, one level into a list or tuple."""
    if isinstance(value, (list, tuple)):
        return [v for v in value if isinstance(v, (int, str)) and not isinstance(v, bool)]
    if isinstance(value, (int, str)) and not isinstance(value, bool):
        return [value]
    return []


# Fixture rows: (address, size, data_name, access, area). access/area None = left out.
X_ROWS = [
    (0, 2, "Model Number", "R", "EEPROM"),
    (6, 1, "Firmware Version", "R", "EEPROM"),
    (11, 1, "Operating Mode", "RW", "EEPROM"),
    (48, 4, "Max Position Limit", "RW", "EEPROM"),
    (52, 4, "Min Position Limit", "RW", "EEPROM"),
    (64, 1, "Torque Enable", "RW", "RAM"),
    (116, 4, "Goal Position", "RW", "RAM"),
    (132, 4, "Present Position", "R", "RAM"),
]
Y_ROWS = [r if r[2] != "Operating Mode" else (33, 1, "Operating Mode", "RW", "EEPROM") for r in X_ROWS]
AX_ROWS = [
    (0, 2, "Model Number", "R", "EEPROM"),
    (2, 1, "Firmware Version", "R", "EEPROM"),
    (6, 2, "CW Angle Limit", "RW", "EEPROM"),
    (8, 2, "CCW Angle Limit", "RW", "EEPROM"),
    (24, 1, "Torque Enable", "RW", "RAM"),
    (30, 2, "Goal Position", "RW", "RAM"),
    (36, 2, "Present Position", "R", "RAM"),
]
XL320_ROWS = AX_ROWS[:4] + [(11, 1, "Control Mode", "RW", "EEPROM")] + AX_ROWS[4:]

X_SCALE = (2048, 0, 4095, -3.1415927, 3.1400587)
AX_SCALE = (512, 0, 1023, -2.6205530, 2.6154347)

X_MODES = "(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])"
Y_MODES = "(bounded: [3], unbounded: [], no_position: [0, 1])"
XL320_MODES = "(bounded: [2], unbounded: [], no_position: [1])"

AX_URL = "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/"
P1_URL = "https://emanual.robotis.com/docs/en/dxl/protocol1/"
P2_URL = "https://emanual.robotis.com/docs/en/dxl/protocol2/"

MODE_NAMES = ("Operating Mode", "Control Mode")
AUTO = object()


def row(
    address: int,
    size: int,
    name: str,
    access: str | None = "RW",
    area: str | None = "RAM",
    span: tuple[int, int] | None = None,
) -> str:
    fields = [f"address: {address}", f"size: {size}", f'data_name: "{name}"']
    if access is not None:
        fields.append(f"access: {access}")
    if area is not None:
        fields.append(f"area: {area}")
    if span is not None:
        fields.append(f"range: (min: {span[0]}, max: {span[1]})")
    return "{ " + ", ".join(fields) + " }"


def table(
    rows: list[tuple] | None = None,
    *,
    source: str | None = "Unverified",
    protocols: str | None = "[P1, P2]",
    instructions: str | None = None,
    operating_modes: object = AUTO,
    type_info: tuple | None = X_SCALE,
    extra: str = "",
) -> str:
    """RON text of one table. `None` leaves a key out.

    `operating_modes` defaults to the mode lists that fit the rows: written
    when the rows have an `Operating Mode` or `Control Mode` row, else left
    out. Pass `None` to leave it out, or a string to write that value.
    """
    rows = X_ROWS if rows is None else rows
    if operating_modes is AUTO:
        names = {r[2]: r for r in rows}
        if "Control Mode" in names:
            operating_modes = XL320_MODES
        elif "Operating Mode" in names:
            operating_modes = Y_MODES if names["Operating Mode"][0] == 33 else X_MODES
        else:
            operating_modes = None
    lines = ["("]
    if source is not None:
        lines.append(f"  source: {source},")
    if protocols is not None:
        lines.append(f"  protocols: {protocols},")
    if instructions is not None:
        lines.append(f"  instructions: {instructions},")
    if operating_modes is not None:
        lines.append(f"  operating_modes: {operating_modes},")
    if type_info is not None:
        zero, lo, hi, rmin, rmax = type_info
        lines.append(
            "  type_info: {\n"
            f"    value_of_zero_radian_position: {zero},\n"
            f"    value_of_max_radian_position: {hi},\n"
            f"    value_of_min_radian_position: {lo},\n"
            f"    min_radian: {rmin},\n"
            f"    max_radian: {rmax},\n"
            "  },"
        )
    if extra:
        lines.append(extra)
    lines.append("  control_table: [")
    lines.extend(f"    {row(*r)}," for r in rows)
    lines.append("  ],")
    lines.append(")")
    return "\n".join(lines) + "\n"


def ref_list(*files: str | tuple[int, str]) -> str:
    items = []
    for f in files:
        from_fw, stem = (0, f) if isinstance(f, str) else f
        items.append(f'(from_fw: {from_fw}, file: "{stem}")')
    return "[" + ", ".join(items) + "]"


def entry(
    stem: str,
    lists: tuple[str, ...] = ("p1", "p2"),
    *,
    family: str = "x-series",
    name: str | None = None,
    default: str | None = AUTO,  # type: ignore[assignment]
    tables: str | None = None,
) -> str:
    """One model entry mapping `stem` under `lists`. `tables` writes a legacy `tables` key (rejected from T2 on)."""
    ref = ref_list(stem)
    parts = [f'family: "{family}"', f'name: "{name or stem.upper()}"']
    parts += [f"{key}: {ref}" for key in lists]
    if default is AUTO:
        default = "P2" if len(lists) == 2 else None
    if default is not None:
        parts.append(f"default: {default}")
    if tables is not None:
        parts.append(f"tables: {tables}")
    return "(" + ", ".join(parts) + ")"


def model_map(entries: dict[int, str], not_mapped: list[str] | None = None) -> str:
    body = "".join(f"    {number}: {text},\n" for number, text in entries.items())
    text = "(\n  models: {\n" + body + "  },\n"
    if not_mapped is not None:
        text += "  not_mapped: [" + ", ".join(not_mapped) + "],\n"
    return text + ")\n"


def clean_files() -> dict[str, str]:
    """A fixture that passes every rule: an X, an AX and an XL-320 table, all mapped."""
    return {
        "x": table(),
        "ax": table(AX_ROWS, protocols="[P1]", type_info=AX_SCALE),
        "xl320": table(XL320_ROWS, protocols="[P2]", type_info=AX_SCALE),
        "dynamixel": model_map(
            {
                12: entry("ax", ("p1",), family="ax"),
                350: entry("xl320", ("p2",)),
                1020: entry("x", name="XM430-W350"),
            },
            [],
        ),
    }


class FixtureCase(unittest.TestCase):
    """Base class: a fresh temporary directory per test."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self._tmp.name)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def check(self, files: dict[str, str], *extra: str) -> check_tables.Outcome:
        write_fixture(self.dir, files)
        return run_check(["--dir", str(self.dir), "--today", TODAY, *extra])

    def check_with(self, *extra: str, **changes: str | None) -> check_tables.Outcome:
        """Runs the check on `clean_files()` with some files replaced (None deletes one)."""
        files = clean_files()
        for stem, text in changes.items():
            if text is None:
                files.pop(stem, None)
            else:
                files[stem] = text
        return self.check(files, *extra)

    def assertFails(self, result: check_tables.Outcome, cause: str, **fields: object) -> check_tables.Diagnostic:
        """Exit 1 (a rule is broken) with a diagnostic of `cause` whose data holds `fields`."""
        self.assertEqual(result.code, 1, f"expected exit 1\nstdout:\n{result.out}\nstderr:\n{result.err}")
        return self.assertDiagnostic(result, cause, **fields)

    def assertDiagnostic(self, result: check_tables.Outcome, cause: str, **fields: object) -> check_tables.Diagnostic:
        """A diagnostic of `cause` whose data holds `fields`; its line names the code and every deciding value."""
        matches = [d for d in result.diagnostics if d.code == cause]
        self.assertTrue(matches, f"no [{cause}] diagnostic\nstderr:\n{result.err}")
        for diagnostic in matches:
            data = diagnostic.data()
            if all(key in data and data[key] == value for key, value in fields.items()):
                break
        else:
            self.fail(f"no [{cause}] diagnostic with {fields}; found {[d.data() for d in matches]}")
        line = diagnostic.line()
        self.assertIn(f"[{cause}]", line)
        for key, value in diagnostic.values.items():
            for scalar in _scalars(value) if key not in ("allowed", "expected") else []:
                self.assertIn(str(scalar), line, f"{key}={value!r} is not in the line {line!r}")
        return diagnostic

    def assertPasses(self, result: check_tables.Outcome) -> None:
        self.assertEqual(result.code, 0, f"expected exit 0\nstdout:\n{result.out}\nstderr:\n{result.err}")
        self.assertEqual(result.diagnostics, [])


ONE_LINE_TABLE = """(
  type_info: {
    value_of_zero_radian_position: 512,
    value_of_max_radian_position: 1023,
    value_of_min_radian_position: 0,
    min_radian: 0.0,
    max_radian: 5.2359877,   // 300° = 5.2359 rad
  },
  unit_info: [
    { data_name: "Torque", value: 0.001757, unit: "N·m/bit", sign_type: "unsigned" },
  ],
  control_table: [
    // EEPROM area
    { address: 0, size: 2, data_name: "Model Number" },
    { address: 2, size: 1, data_name: "Firmware Version" },
    // RAM area
    { address: 24, size: 1, data_name: "Torque Enable" },
  ],
)
"""

MULTI_LINE_TABLE = """(
  type_info: {
    value_of_zero_radian_position: 2048,
    value_of_max_radian_position: 4095,
    value_of_min_radian_position: 0,
    min_radian: -3.14159265,
    max_radian: 3.14159265,
  },
  control_table: [
    {
      address: 0,
      size: 2,
      data_name: "Model Number",
    },
    {
      address: 11,
      size: 1,
      data_name: "Operating Mode",
    },
  ],
)
"""


class ParserTests(FixtureCase):
    """RON-subset parser (research R-6), FR-012."""

    def test_paren_and_brace_records_are_dicts(self) -> None:
        self.assertEqual(parse_ron('( a: 1, b: "x" )'), {"a": 1, "b": "x"})
        self.assertEqual(
            parse_ron('{ address: 0, size: 2, data_name: "Model Number" }'),
            {"address": 0, "size": 2, "data_name": "Model Number"},
        )
        self.assertEqual(parse_ron("()"), {})

    def test_integer_keyed_map(self) -> None:
        value = parse_ron('{ 12: (name: "AX-12A"), 18: (name: "AX-18A") }')
        self.assertEqual(value, {12: {"name": "AX-12A"}, 18: {"name": "AX-18A"}})

    def test_lists_with_trailing_commas(self) -> None:
        self.assertEqual(parse_ron("[1, 2, 3,]"), [1, 2, 3])
        self.assertEqual(parse_ron("[]"), [])
        self.assertEqual(parse_ron("( a: [ (x: 1), ], )"), {"a": [{"x": 1}]})

    def test_strings_with_escapes_and_non_ascii(self) -> None:
        self.assertEqual(parse_ron(r'"a\"b\\c\nd"'), 'a"b\\c\nd')
        self.assertEqual(parse_ron('"N·m/bit"'), "N·m/bit")
        self.assertEqual(parse_ron('"300°"'), "300°")

    def test_numbers(self) -> None:
        self.assertEqual(parse_ron("42"), 42)
        self.assertEqual(parse_ron("-7"), -7)
        value = parse_ron("-3.1415927")
        self.assertIsInstance(value, float)
        self.assertAlmostEqual(value, -3.1415927)
        self.assertEqual(parse_ron("0.0"), 0.0)
        self.assertIsInstance(parse_ron("0.0"), float)
        self.assertAlmostEqual(parse_ron("1e-3"), 0.001)

    def test_comments_anywhere(self) -> None:
        text = """// head
        ( // after paren
          a: 1, // after value
          // own line
          b: [ 1, // in list
               2 ],
        ) // tail"""
        self.assertEqual(parse_ron(text), {"a": 1, "b": [1, 2]})

    def test_bare_identifiers(self) -> None:
        value = parse_ron("( source: Unverified, protocols: [P1, P2], access: RW, area: EEPROM )")
        self.assertEqual(value["source"], Ident("Unverified"))
        self.assertEqual(value["protocols"], [Ident("P1"), Ident("P2")])
        self.assertEqual(value["access"], Ident("RW"))
        self.assertEqual(value["area"], Ident("EEPROM"))
        self.assertNotEqual(Ident("R"), Ident("RW"))
        self.assertNotEqual(Ident("R"), "R")

    def test_tagged_values(self) -> None:
        value = parse_ron(
            'Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified: "2026-10-28")'
        )
        self.assertIsInstance(value, Tagged)
        self.assertEqual(value.name, "Verified")
        self.assertEqual(
            value.args,
            {"url": "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", "verified": "2026-10-28"},
        )
        same = parse_ron("SameNumberAs(12)")
        self.assertEqual(same, Tagged("SameNumberAs", [12]))

    def test_one_line_rows(self) -> None:
        table = parse_ron(ONE_LINE_TABLE)
        self.assertEqual(
            table["control_table"][0], {"address": 0, "size": 2, "data_name": "Model Number"}
        )
        self.assertEqual(len(table["control_table"]), 3)
        self.assertEqual(table["unit_info"][0]["unit"], "N·m/bit")
        self.assertAlmostEqual(table["type_info"]["max_radian"], 5.2359877)

    def test_multi_line_rows(self) -> None:
        table = parse_ron(MULTI_LINE_TABLE)
        self.assertEqual(
            table["control_table"][1], {"address": 11, "size": 1, "data_name": "Operating Mode"}
        )
        self.assertEqual(table["type_info"]["min_radian"], -3.14159265)

    def test_duplicate_map_key_names_file_and_key(self) -> None:
        with self.assertRaises(RonError) as ctx:
            parse_ron("{ 1020: (a: 1), 1020: (a: 2) }", "dynamixel.ron")
        message = str(ctx.exception)
        self.assertIn("dynamixel.ron", message)
        self.assertIn("1020", message)
        self.assertIn("duplicate", message)

    def test_duplicate_record_key(self) -> None:
        with self.assertRaises(RonError) as ctx:
            parse_ron("( a: 1, a: 2 )", "t.ron")
        self.assertIn("`a`", str(ctx.exception))

    def test_unterminated_string_names_file_and_line(self) -> None:
        with self.assertRaises(RonError) as ctx:
            parse_ron('(\n  a: 1,\n  b: "open\n)\n', "broken.ron")
        err = ctx.exception
        self.assertEqual(err.file, "broken.ron")
        self.assertEqual(err.line, 3)
        self.assertIn("broken.ron:3", str(err))

    def test_trailing_garbage_is_an_error(self) -> None:
        with self.assertRaises(RonError):
            parse_ron("( a: 1 ) )")


    def test_cli_duplicate_key_exits_2(self) -> None:
        code, _out, err, _diagnostics = self.check(
            {
                "dynamixel": '(models: { 1020: (family: "x-series", name: "A"), 1020: (family: "x-series", name: "B") })',
            }
        )
        self.assertEqual(code, 2)
        self.assertIn("dynamixel.ron", err)
        self.assertIn("1020", err)

    def test_cli_unterminated_string_exits_2(self) -> None:
        code, _out, err, _diagnostics = self.check(
            {"dynamixel": "(models: {})", "broken": '(\n  source: "open\n)\n'}
        )
        self.assertEqual(code, 2)
        self.assertIn("broken.ron:2", err)


class CliTests(FixtureCase):
    """Report frame, exit codes and options (contract table-tools section 1)."""

    def test_clean_fixture_exits_0(self) -> None:
        self.assertPasses(self.check(clean_files()))

    def test_missing_map_exits_2(self) -> None:
        code, _out, err, _diagnostics = self.check_with(dynamixel=None)
        self.assertEqual(code, 2)
        self.assertIn("dynamixel.ron", err)

    def test_unparseable_file_exits_2_naming_it(self) -> None:
        code, _out, err, _diagnostics = self.check_with(x="( source: Unverified, control_table: [ { address: 0 ")
        self.assertEqual(code, 2)
        self.assertIn("x.ron", err)

    def test_bad_arguments_exit_2(self) -> None:
        for argv in (["--today", "28.10.2026"], ["--today", "2026-02-30"], ["--bogus"]):
            with self.subTest(argv=argv):
                self.assertEqual(run_check(["--dir", str(self.dir), *argv]).code, 2)

    def test_sections_in_order(self) -> None:
        code, out, _err, _diagnostics = self.check(clean_files())
        self.assertEqual(code, 0)
        heads = [line for line in out.splitlines() if line and not line.startswith(" ")]
        self.assertEqual(heads[:3], ["tables", "dual-protocol models", "errors"])
        self.assertTrue(heads[3].startswith("summary: 3 tables,"), heads)
        self.assertEqual(len(heads), 4)
        self.assertIn("0 errors", heads[3])

    def test_errors_go_to_stdout_and_stderr(self) -> None:
        result = self.check_with(x="( source: Unverified, protocols: [P1, P2] )")
        self.assertFails(result, "control-table-missing", file="x.ron", field="control_table")
        section = result.out.split("errors\n", 1)[1].split("summary:", 1)[0]
        error_lines = [line.strip() for line in section.splitlines() if line.strip()]
        self.assertTrue(error_lines)
        self.assertEqual(error_lines, [line.strip() for line in result.err.splitlines() if line.strip()])
        self.assertTrue(all(line.startswith(("x.ron", "hint: ")) for line in error_lines), error_lines)
        self.assertIn(f"{len(result.diagnostics)} errors", result.out)

    def test_line_names_file_location_and_cause(self) -> None:
        result = self.check_with(x=table(replaced(X_ROWS, "Goal Position", access=None)))
        diagnostic = self.assertFails(
            result, "row-access-missing", file="x.ron", location='row 116 "Goal Position"', register="Goal Position"
        )
        self.assertEqual(diagnostic.line(), 'x.ron: row 116 "Goal Position": [row-access-missing] missing `access`')
        self.assertIn(diagnostic.line(), result.err.splitlines())
        self.assertIsNone(diagnostic.hint)

    def test_a_hint_follows_its_line_and_stays_out_of_the_message(self) -> None:
        files = unmapped()
        files["extra"] = table()
        result = self.check(files)
        diagnostic = self.assertFails(result, "table-unlisted", file="extra.ron", table="extra")
        self.assertTrue(diagnostic.hint)
        self.assertNotIn(diagnostic.hint, diagnostic.message)
        lines = result.err.splitlines()
        at = lines.index(diagnostic.line())
        self.assertEqual(lines[at + 1], f"  hint: {diagnostic.hint}")
        self.assertIn(f"\n    hint: {diagnostic.hint}\n", result.out)

    def test_main_prints_the_outcome(self) -> None:
        write_fixture(self.dir, clean_files())
        write_fixture(self.dir, {"x": "( source: Unverified, protocols: [P1, P2] )"})
        argv = ["--dir", str(self.dir), "--today", TODAY]
        outcome = run_check(argv)
        self.assertEqual(run_main(argv), (outcome.code, outcome.out, outcome.err))

    def test_stems_limit_only_the_table_section(self) -> None:
        files = clean_files()
        files["x"] = "( source: Unverified, protocols: [P1, P2] )"
        code, out, _err, _diagnostics = self.check(files, "ax")
        self.assertEqual(code, 1)
        section = out.split("dual-protocol models", 1)[0]
        self.assertIn("  ax ", section)
        self.assertNotIn("xl320", section)
        self.assertIn("x.ron", out.split("errors\n", 1)[1])

    def test_unknown_stem_exits_2(self) -> None:
        self.assertEqual(self.check(clean_files(), "nope").code, 2)

    def test_markdown_prints_a_list(self) -> None:
        code, out, _err, _diagnostics = self.check(clean_files(), "--markdown", "ax", "x")
        self.assertEqual(code, 0)
        lines = [line for line in out.splitlines() if line]
        self.assertEqual(len(lines), 2)
        self.assertTrue(all(line.startswith("- `") for line in lines), lines)
        self.assertTrue(lines[0].startswith("- `ax`"))


class CauseRegistryTests(unittest.TestCase):
    """Every cause code is registered once, used by the check and tested here (FR-6.14, NFR-12)."""

    def test_codes_are_short_stable_identifiers(self) -> None:
        for code in check_tables.CAUSES:
            with self.subTest(code=code):
                self.assertRegex(code, r"^[a-z0-9]+(-[a-z0-9]+)*$")
                self.assertLessEqual(len(code), 40)

    def test_every_code_is_used_by_the_check(self) -> None:
        source = Path(check_tables.__file__).read_text(encoding="utf-8")
        unused = [code for code in check_tables.CAUSES if source.count(f'"{code}"') < 2]
        self.assertEqual(unused, [])

    def test_every_code_has_a_test(self) -> None:
        tests = Path(__file__).read_text(encoding="utf-8")
        untested = [code for code in check_tables.CAUSES if f'"{code}"' not in tests]
        self.assertEqual(untested, [])

    def test_every_code_has_a_documented_exit_code(self) -> None:
        for code, cause in check_tables.CAUSES.items():
            with self.subTest(code=code):
                self.assertIn(f"\n    {cause.exit_code}  ", check_tables.__doc__)

    def test_an_unregistered_code_is_refused(self) -> None:
        with self.assertRaises(KeyError):
            check_tables.Report().add("no-such-cause", "x.ron", "", "message")


def verified(url: str = AX_URL, date: str = TODAY) -> str:
    return f'Verified(url: "{url}", verified: "{date}")'


def ax_instructions() -> str:
    return f'[(protocol: P1, source: "{P1_URL}", list: [Ping, Read, Write])]'


class SourceTests(FixtureCase):
    """Verification flag: US1 AS1–AS4, FR-001, FR-002, data-model "Verification flag"."""

    def ax(self, **kw: object) -> str:
        args: dict = {"protocols": "[P1]", "type_info": AX_SCALE}
        args.update(kw)
        return table(AX_ROWS, **args)

    def verified_ax(self, source: str) -> str:
        return self.ax(source=source, instructions=ax_instructions())

    def test_missing_source_fails_naming_the_file(self) -> None:  # AS2
        self.assertFails(self.check_with(ax=self.ax(source=None)), "source-missing", file="ax.ron", field="source")

    def test_unverified_is_listed_not_an_error(self) -> None:  # AS3
        code, out, _err, _diagnostics = self.check_with()
        self.assertEqual(code, 0)
        self.assertRegex(out, r"\n  ax\s+unverified\s")

    def test_verified_lists_url_and_date(self) -> None:  # AS1
        result = self.check_with(ax=self.verified_ax(verified()))
        self.assertPasses(result)
        self.assertIn(f"verified {AX_URL} {TODAY}", result.out)

    def test_url_on_another_host_fails(self) -> None:  # AS4
        for url in ("https://example.com/ax-12a/", "http://emanual.robotis.com/docs/en/dxl/ax/ax-12a/"):
            with self.subTest(url=url):
                self.assertFails(
                    self.check_with(ax=self.verified_ax(verified(url=url))),
                    "url-not-emanual",
                    file="ax.ron",
                    field="source.url",
                    found=url,
                    expected=check_tables.EMANUAL_PREFIX,
                )

    def test_url_must_be_a_string(self) -> None:
        source = f'Verified(url: 3, verified: "{TODAY}")'
        self.assertFails(self.check_with(ax=self.verified_ax(source)), "url-not-string", field="source.url", found=3)

    def test_impossible_date_fails(self) -> None:
        self.assertFails(
            self.check_with(ax=self.verified_ax(verified(date="2026-02-30"))),
            "source-date-not-real",
            file="ax.ron",
            field="source.verified",
            found="2026-02-30",
        )

    def test_future_date_fails(self) -> None:
        self.assertFails(
            self.check_with(ax=self.verified_ax(verified(date="2026-10-29"))),
            "source-date-future",
            file="ax.ron",
            field="source.verified",
            found="2026-10-29",
            today=TODAY,
        )

    def test_non_iso_date_fails(self) -> None:
        self.assertFails(
            self.check_with(ax=self.verified_ax(verified(date="28.10.2026"))),
            "source-date-not-iso",
            file="ax.ron",
            field="source.verified",
            found="28.10.2026",
        )

    def test_missing_verified_field_fails(self) -> None:
        self.assertFails(
            self.check_with(ax=self.verified_ax(f'Verified(url: "{AX_URL}")')),
            "source-date-missing",
            file="ax.ron",
            field="source.verified",
        )

    def test_missing_url_fails(self) -> None:
        self.assertFails(
            self.check_with(ax=self.verified_ax(f'Verified(verified: "{TODAY}")')),
            "source-url-missing",
            file="ax.ron",
            field="source.url",
        )

    def test_unknown_verified_field_fails(self) -> None:
        source = f'Verified(url: "{AX_URL}", verified: "{TODAY}", by: "me")'
        self.assertFails(
            self.check_with(ax=self.verified_ax(source)), "source-unknown-field", field="source", found=["by"]
        )

    def test_other_tag_fails(self) -> None:
        self.assertFails(
            self.check_with(ax=self.ax(source="Checked")), "source-shape", file="ax.ron", field="source",
            found=Ident("Checked"),
        )
        self.assertFails(
            self.check_with(ax=self.ax(source='"Unverified"')), "source-shape", field="source", found="Unverified"
        )

    def test_markdown_shows_flag_url_and_date(self) -> None:
        code, out, _err, _diagnostics = self.check_with("--markdown", "ax", "x", ax=self.verified_ax(verified()))
        self.assertEqual(code, 0, out)
        self.assertIn(f"- `ax`: verified {AX_URL} {TODAY}", out)
        self.assertIn("- `x`: unverified", out)


def instr(protocol: str = "P1", names: str = "Ping, Read, Write", source: str = P1_URL, from_fw: int | None = None) -> str:
    fw = f", from_fw: {from_fw}" if from_fw is not None else ""
    return f'(protocol: {protocol}, source: "{source}", list: [{names}]{fw})'


class InstructionTests(FixtureCase):
    """`instructions`: US2 AS1, FR-006, data-model "Instruction entry"."""

    def x(self, entries: list[str] | None, *, source: str | None = None) -> str:
        source = source or verified("https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/")
        body = None if entries is None else "[" + ", ".join(entries) + "]"
        return table(source=source, instructions=body)

    def both(self) -> list[str]:
        return [instr("P1"), instr("P2", "Ping, Read, Write, SyncRead, FastSyncRead", P2_URL)]

    def test_valid_entries_pass(self) -> None:
        self.assertPasses(self.check_with(x=self.x(self.both())))

    def test_instructions_on_unverified_table_fail(self) -> None:
        self.assertFails(
            self.check_with(x=self.x(self.both(), source="Unverified")),
            "instructions-on-unverified",
            file="x.ron",
            field="instructions",
            source=Ident("Unverified"),
        )

    def test_verified_table_needs_an_entry_per_protocol(self) -> None:
        self.assertFails(
            self.check_with(x=self.x([instr("P1")])), "instructions-protocol-missing", file="x.ron", protocol="P2"
        )
        diagnostic = self.assertFails(
            self.check_with(x=self.x(None)), "instructions-missing", file="x.ron", field="instructions"
        )
        self.assertTrue(diagnostic.hint)

    def test_instructions_must_be_a_non_empty_list(self) -> None:
        self.assertFails(
            self.check_with(x=self.x([])), "instructions-shape", file="x.ron", field="instructions", found="an empty list"
        )

    def test_protocol_must_be_in_protocols(self) -> None:
        ax = table(
            AX_ROWS,
            protocols="[P1]",
            type_info=AX_SCALE,
            source=verified(),
            instructions="[" + instr("P1") + ", " + instr("P2", source=P2_URL) + "]",
        )
        self.assertFails(
            self.check_with(ax=ax), "instruction-protocol-not-in-table", file="ax.ron", index=1, found="P2",
            allowed=["P1"],
        )

    def test_protocol_must_be_p1_or_p2(self) -> None:
        self.assertFails(
            self.check_with(x=self.x([instr("P3"), *self.both()])),
            "instruction-protocol-invalid", index=0, field="protocol", found=Ident("P3"),
        )

    def test_source_must_be_on_the_emanual(self) -> None:
        self.assertFails(
            self.check_with(x=self.x([instr("P1", source="https://github.com/ROBOTIS-GIT/"), self.both()[1]])),
            "url-not-emanual",
            file="x.ron",
            field="instructions.source",
            index=0,
            found="https://github.com/ROBOTIS-GIT/",
        )

    def test_list_non_empty_known_names_no_duplicates(self) -> None:
        p2 = self.both()[1]
        cases = (
            ("", "instruction-list-shape", {"field": "list", "found": "an empty list"}),
            ("Ping, Ping", "instruction-duplicate", {"found": "Ping"}),
            ("Ping, Teleport", "instruction-unknown", {"found": Ident("Teleport")}),
            ('"Ping"', "instruction-unknown", {"found": "Ping"}),
        )
        for names, cause, fields in cases:
            with self.subTest(names=names):
                self.assertFails(self.check_with(x=self.x([instr("P1", names), p2])), cause, file="x.ron", index=0, **fields)

    def test_p1_entries_only_use_protocol_1_instructions(self) -> None:
        p2 = self.both()[1]
        self.assertFails(
            self.check_with(x=self.x([instr("P1", "Ping, SyncRead"), p2])),
            "instruction-not-p1", file="x.ron", index=0, found="SyncRead",
            allowed=check_tables.P1_INSTRUCTIONS,
        )
        self.assertPasses(
            self.check_with(
                x=self.x(
                    [instr("P1", "Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, SyncWrite, BulkRead"), p2]
                )
            )
        )

    def test_entries_of_one_protocol_differ_in_from_fw(self) -> None:
        p2 = self.both()[1]
        self.assertFails(
            self.check_with(x=self.x([instr("P1"), instr("P1", "Reboot"), p2])),
            "instruction-from-fw-duplicate", file="x.ron", index=1, protocol="P1", found=None,
        )
        self.assertPasses(self.check_with(x=self.x([instr("P1"), instr("P1", "Reboot", from_fw=40), p2])))
        self.assertFails(
            self.check_with(x=self.x([instr("P1"), instr("P1", "Reboot", from_fw=40), instr("P1", "BulkRead", from_fw=40), p2])),
            "instruction-from-fw-duplicate", index=2, protocol="P1", found=40,
        )

    def test_entry_must_be_a_record_with_known_fields(self) -> None:
        p2 = self.both()[1]
        self.assertFails(
            self.check_with(x=self.x(["P1", p2])), "instruction-entry-not-record", file="x.ron", index=0,
            found="identifier `P1`",
        )
        self.assertFails(
            self.check_with(x=self.x(['(protocol: P1, list: [Ping])', p2])),
            "instruction-source-missing", index=0, field="source",
        )
        self.assertFails(
            self.check_with(x=self.x([instr("P1", from_fw=300), p2])),
            "instruction-from-fw-invalid", index=0, field="from_fw", found=300, allowed=(0, 255),
        )
        unknown = f'(protocol: P1, source: "{P1_URL}", list: [Ping], note: "x")'
        self.assertFails(
            self.check_with(x=self.x([unknown, p2])), "instruction-entry-unknown-field", index=0, found=["note"]
        )


def without(rows: list[tuple], *names: str) -> list[tuple]:
    return [r for r in rows if r[2] not in names]


def replaced(rows: list[tuple], name: str, **fields: object) -> list[tuple]:
    keys = ("address", "size", "name", "access", "area")
    out = []
    for r in rows:
        if r[2] == name:
            values = dict(zip(keys, r))
            values.update(fields)
            r = tuple(values[k] for k in keys)
        out.append(r)
    return out


class ProtocolsTests(FixtureCase):
    """`protocols`: US3 AS1, FR-005, xtask check 9."""

    def test_missing_empty_duplicate_or_unknown_fails(self) -> None:
        cases = (
            (None, "protocols-missing", {}),
            ("[]", "protocols-empty", {"found": "an empty list"}),
            ("[P1, P1]", "protocol-duplicate", {"found": "P1"}),
            ("[P3]", "protocol-unknown", {"found": Ident("P3"), "allowed": ("P1", "P2")}),
            ("P1", "protocols-not-list", {"found": Ident("P1")}),
            ("[P1, P2, P1]", "protocol-duplicate", {"found": "P1"}),
        )
        for protocols, cause, fields in cases:
            with self.subTest(protocols=protocols):
                self.assertFails(
                    self.check_with(x=table(protocols=protocols)), cause, file="x.ron", field="protocols", **fields
                )

    def test_column_shows_protocols(self) -> None:
        code, out, _err, _diagnostics = self.check_with()
        self.assertEqual(code, 0)
        self.assertRegex(out, r"\n  x\s+unverified\s+\[P1, P2\]\s")
        self.assertRegex(out, r"\n  xl320\s+unverified\s+\[P2\]\s")
        self.assertRegex(out, r"\n  ax\s+unverified\s+\[P1\]\s")


class ShapeTests(FixtureCase):
    """The table record, `control_table` and each row's address, size and name (ron-schema §1)."""

    def test_the_file_must_be_a_record(self) -> None:
        self.assertFails(self.check(unmapped(extra="[1, 2]")), "table-not-record", file="extra.ron", found="a list")

    def test_control_table_must_be_a_non_empty_list(self) -> None:
        self.assertFails(
            self.check(unmapped(extra=table([]))), "control-table-shape", file="extra.ron", field="control_table",
            found="an empty list",
        )
        self.assertFails(
            self.check(unmapped(extra=table().replace("control_table: [", "control_table: 3, other: ["))),
            "control-table-shape", found="integer 3",
        )

    def test_a_row_must_be_a_record(self) -> None:
        text = table().replace(row(*X_ROWS[6]), '"Goal Position"')
        self.assertFails(
            self.check_with(x=text), "row-not-record", file="x.ron", field="control_table", index=6,
            found="string 'Goal Position'",
        )

    def test_row_address_size_and_name(self) -> None:
        cases = (
            (replaced(X_ROWS, "Present Position", address=70000), "row-address-invalid",
             {"register": "Present Position", "field": "address", "found": 70000, "allowed": (0, 65535)}),
            (replaced(X_ROWS, "Present Position", address='"a"'), "row-address-invalid",
             {"register": "Present Position", "found": "a"}),
            (replaced(X_ROWS, "Present Position", size=300), "row-size-invalid",
             {"register": "Present Position", "field": "size", "found": 300, "allowed": (1, 255)}),
            (replaced(X_ROWS, "Present Position", size=0), "row-size-invalid", {"found": 0}),
            (X_ROWS + [(200, 1, "", "RW", "RAM")], "row-name-invalid", {"field": "data_name", "found": ""}),
        )
        for rows, cause, fields in cases:
            with self.subTest(cause=cause, fields=fields):
                self.assertFails(self.check_with(x=table(rows)), cause, file="x.ron", **fields)


class RowTests(FixtureCase):
    """Row `access` and `area`: US3 AS2, FR-007, data-model "Register row"."""

    def test_missing_access_or_area_names_the_row(self) -> None:
        for key in ("access", "area"):
            with self.subTest(key=key):
                rows = replaced(X_ROWS, "Goal Position", **{key: None})
                self.assertFails(
                    self.check_with(x=table(rows)), f"row-{key}-missing", file="x.ron",
                    location='row 116 "Goal Position"', register="Goal Position", field=key,
                )

    def test_other_access_or_area_values_fail(self) -> None:
        self.assertFails(
            self.check_with(x=table(replaced(X_ROWS, "Present Position", access="W"))),
            "row-access-invalid", file="x.ron", register="Present Position", field="access", found=Ident("W"),
            allowed=("R", "RW"),
        )
        self.assertFails(
            self.check_with(x=table(replaced(X_ROWS, "Present Position", area="ROM"))),
            "row-area-invalid", file="x.ron", register="Present Position", field="area", found=Ident("ROM"),
            allowed=("EEPROM", "RAM", "Hybrid"),
        )

    def test_unknown_row_field_fails(self) -> None:
        text = table().replace('data_name: "Goal Position"', 'data_name: "Goal Position", colour: 1')
        self.assertFails(
            self.check_with(x=text), "row-unknown-field", file="x.ron", register="Goal Position", found=["colour"]
        )

    def test_hybrid_area_is_allowed(self) -> None:
        rows = X_ROWS + [(212, 4, "Velocity I Gain", "RW", "Hybrid")]
        self.assertPasses(self.check_with(x=table(rows)))

    def test_hybrid_is_not_a_valid_area_for_a_role(self) -> None:
        rows = replaced(X_ROWS, "Goal Position", area="Hybrid")
        self.assertFails(
            self.check_with(x=table(rows)), "role-attributes", file="x.ron", register="Goal Position",
            found=("RW", "Hybrid"), expected=("RW", "RAM"),
        )

    def test_rows_are_checked_in_unmapped_tables_too(self) -> None:
        rows = replaced(X_ROWS, "Goal Position", area=None)
        self.assertFails(self.check(unmapped(extra=table(rows))), "row-area-missing", file="extra.ron", field="area")


class RoleTests(FixtureCase):
    """Role rules on mapped tables: contract 4.3, xtask checks 10–12, 15, 16."""

    def test_missing_role_fails(self) -> None:
        for name in ("Torque Enable", "Goal Position", "Present Position", "Firmware Version", "Model Number"):
            with self.subTest(name=name):
                self.assertFails(self.check_with(x=table(without(X_ROWS, name))), "role-missing", file="x.ron", register=name)

    def test_exactly_one_complete_limit_form(self) -> None:
        both = AX_ROWS[:2] + X_ROWS[3:5] + AX_ROWS[2:]
        self.assertFails(
            self.check_with(ax=table(both, protocols="[P1]", type_info=AX_SCALE)), "limit-forms-both", file="ax.ron"
        )
        half = table(without(AX_ROWS, "CCW Angle Limit"), protocols="[P1]", type_info=AX_SCALE)
        self.assertFails(
            self.check_with(ax=half), "limit-pair-half", file="ax.ron", register="CW Angle Limit",
            missing="CCW Angle Limit",
        )
        self.assertFails(self.check_with(ax=half), "limit-pair-missing", file="ax.ron")
        self.assertFails(
            self.check_with(x=table(without(X_ROWS, "Min Position Limit"))), "limit-pair-half", file="x.ron",
            register="Max Position Limit", missing="Min Position Limit",
        )

    def test_role_attributes(self) -> None:
        cases = [
            ("Torque Enable", {"access": "R"}, ("R", "RAM"), ("RW", "RAM")),
            ("Torque Enable", {"area": "EEPROM"}, ("RW", "EEPROM"), ("RW", "RAM")),
            ("Goal Position", {"access": "R"}, ("R", "RAM"), ("RW", "RAM")),
            ("Goal Position", {"area": "EEPROM"}, ("RW", "EEPROM"), ("RW", "RAM")),
            ("Present Position", {"access": "RW"}, ("RW", "RAM"), ("R", "RAM")),
            ("Present Position", {"area": "EEPROM"}, ("R", "EEPROM"), ("R", "RAM")),
            ("Model Number", {"access": "RW"}, ("RW", "EEPROM"), ("R", "EEPROM")),
            ("Model Number", {"area": "RAM"}, ("R", "RAM"), ("R", "EEPROM")),
        ]
        for name, change, found, expected in cases:
            with self.subTest(name=name, change=change):
                self.assertFails(
                    self.check_with(x=table(replaced(X_ROWS, name, **change))), "role-attributes", file="x.ron",
                    register=name, found=found, expected=expected,
                )

    def test_limit_attributes(self) -> None:
        for name, change, found in (
            ("Max Position Limit", {"access": "R"}, ("R", "EEPROM")),
            ("Min Position Limit", {"area": "RAM"}, ("RW", "RAM")),
        ):
            with self.subTest(name=name):
                self.assertFails(
                    self.check_with(x=table(replaced(X_ROWS, name, **change))), "limit-attributes", file="x.ron",
                    register=name, found=found, expected=("RW", "EEPROM"),
                )
        ax = table(replaced(AX_ROWS, "CW Angle Limit", access="R"), protocols="[P1]", type_info=AX_SCALE)
        self.assertFails(self.check_with(ax=ax), "limit-attributes", file="ax.ron", register="CW Angle Limit")

    def test_model_number_at_0_size_2(self) -> None:
        for change, found in (({"address": 4}, (4, 2)), ({"size": 4}, (0, 4))):
            with self.subTest(change=change):
                self.assertFails(
                    self.check_with(x=table(replaced(X_ROWS, "Model Number", **change))), "model-number-place",
                    file="x.ron", register="Model Number", found=found, expected=(0, 2),
                )

    def test_scale_sanity(self) -> None:
        cases = (
            ((2048, 4095, 0, -3.1415927, 3.1400587), "type-info-max-not-above-min", {"max": 0, "min": 4095}),
            ((2048, 0, 4095, 3.1400587, -3.1415927), "type-info-max-radian-not-above-min",
             {"max": -3.1415927, "min": 3.1400587}),
            ((5000, 0, 4095, -3.1415927, 3.1400587), "type-info-zero-outside", {"found": 5000, "allowed": (0, 4095)}),
            ((-1, 0, 4095, -3.1415927, 3.1400587), "type-info-zero-outside", {"found": -1, "allowed": (0, 4095)}),
        )
        for scale, cause, fields in cases:
            with self.subTest(scale=scale):
                self.assertFails(self.check_with(x=table(type_info=scale)), cause, file="x.ron", field="type_info", **fields)
        self.assertFails(self.check_with(x=table(type_info=None)), "type-info-missing", file="x.ron", field="type_info")

    def test_type_info_shape(self) -> None:
        good = "value_of_zero_radian_position: 2048, value_of_max_radian_position: 4095, value_of_min_radian_position: 0"
        cases = (
            ("[1]", "type-info-not-record", {"found": "a list"}),
            ("{ value_of_zero_radian_position: 2048 }", "type-info-key-missing",
             {"missing": ["value_of_min_radian_position", "value_of_max_radian_position", "min_radian", "max_radian"]}),
            ("{ value_of_zero_radian_position: 2048.0, value_of_max_radian_position: 4095, "
             "value_of_min_radian_position: 0, min_radian: -3.1, max_radian: 3.1 }",
             "type-info-position-not-int", {"found": (2048.0, 0, 4095)}),
            (f'{{ {good}, min_radian: "a", max_radian: 3.1 }}', "type-info-radian-not-number", {"found": ("a", 3.1)}),
        )
        for text, cause, fields in cases:
            with self.subTest(text=text):
                text = table(type_info=None, extra=f"  type_info: {text},")
                self.assertFails(self.check_with(x=text), cause, file="x.ron", field="type_info", **fields)

    def test_duplicate_register_constant(self) -> None:
        rows = X_ROWS + [(200, 4, "Goal-Position", "RW", "RAM")]
        self.assertFails(
            self.check_with(x=table(rows)), "register-constant-duplicate", file="x.ron", register="Goal-Position",
            found="GOAL_POSITION", other="Goal Position",
        )
        rows = X_ROWS + [(116, 4, "Goal Position Alias", "RW", "RAM")]
        self.assertPasses(self.check_with(x=table(rows)))

    def test_unmapped_table_is_not_role_checked(self) -> None:
        rows = without(X_ROWS, "Goal Position", "Torque Enable") + [(200, 4, "Goal-Position", "RW", "RAM")]
        rows = rows + [(204, 4, "Goal Position", "RW", "RAM")]
        self.assertPasses(self.check(unmapped(extra=table(rows, type_info=None))))


def unmapped(**tables: str) -> dict[str, str]:
    """`clean_files()` plus `tables`, each recorded in `not_mapped` as `NoModelNumber`."""
    files = clean_files()
    files.update(tables)
    files["dynamixel"] = model_map(
        {12: entry("ax", ("p1",), family="ax"), 350: entry("xl320", ("p2",)), 1020: entry("x", name="XM430-W350")},
        [f'(file: "{stem}", reason: NoModelNumber)' for stem in tables],
    )
    return files


class ModeSourceTests(FixtureCase):
    """Mode source and `operating_modes`: US3 AS5, AS7, US5 AS6, FR-020, SC-008, xtask checks 11, 14, 17."""

    def ax(self, rows: list[tuple] = AX_ROWS, **kw: object) -> str:
        return table(rows, protocols="[P1]", type_info=AX_SCALE, **kw)

    # passing shapes
    def test_x_y_xl320_and_ax_shapes_pass(self) -> None:
        self.assertPasses(self.check_with(x=table(Y_ROWS)))
        self.assertPasses(self.check_with())

    def test_all_three_lists_empty_passes(self) -> None:
        self.assertPasses(self.check_with(x=table(operating_modes="(bounded: [], unbounded: [], no_position: [])")))

    # rows
    def test_min_max_table_without_operating_mode_has_no_mode_source(self) -> None:
        rows = without(X_ROWS, "Operating Mode")
        self.assertFails(self.check_with(x=table(rows, operating_modes=None)), "mode-source-missing", file="x.ron")

    def test_both_mode_names_fail(self) -> None:
        rows = X_ROWS + [(12, 1, "Control Mode", "RW", "EEPROM")]
        self.assertFails(
            self.check_with(x=table(rows, operating_modes=X_MODES)), "mode-rows-both", file="x.ron",
            found=["Operating Mode", "Control Mode"],
        )

    def test_operating_mode_beside_cw_ccw_pair_fails(self) -> None:
        rows = AX_ROWS + [(11, 1, "Operating Mode", "RW", "EEPROM")]
        self.assertFails(
            self.check_with(ax=self.ax(rows, operating_modes=X_MODES)), "mode-row-beside-angle-limits", file="ax.ron",
            register="Operating Mode",
        )

    def test_two_operating_mode_rows_in_unmapped_table_fail(self) -> None:
        rows = X_ROWS + [(12, 1, "Operating Mode", "RW", "EEPROM")]
        self.assertFails(
            self.check(unmapped(extra=table(rows))), "mode-row-duplicate", file="extra.ron", register="Operating Mode",
            count=2,
        )
        rows = XL320_ROWS + [(12, 1, "Control Mode", "RW", "EEPROM")]
        self.assertFails(
            self.check_with(xl320=table(rows, protocols="[P2]", type_info=AX_SCALE)), "mode-row-duplicate",
            file="xl320.ron", register="Control Mode", count=2,
        )

    def test_control_mode_only_in_xl320(self) -> None:
        files = unmapped(xl330_x=table(XL320_ROWS, protocols="[P2]", type_info=AX_SCALE))
        diagnostic = self.assertFails(
            self.check(files), "control-mode-outside-xl320", file="xl330_x.ron", register="Control Mode"
        )
        self.assertTrue(diagnostic.hint)

    def test_mode_row_attributes(self) -> None:
        for change, found in (
            ({"size": 2}, (2, "RW", "EEPROM")),
            ({"access": "R"}, (1, "R", "EEPROM")),
            ({"area": "RAM"}, (1, "RW", "RAM")),
        ):
            with self.subTest(change=change):
                rows = replaced(X_ROWS, "Operating Mode", **change)
                for result, file in ((self.check_with(x=table(rows)), "x.ron"), (self.check(unmapped(extra=table(rows))), "extra.ron")):
                    self.assertFails(
                        result, "mode-row-attributes", file=file, register="Operating Mode", found=found,
                        expected=(1, "RW", "EEPROM"),
                    )

    def test_p1_mode_row_above_255_fails(self) -> None:
        rows = replaced(X_ROWS, "Operating Mode", address=300)
        diagnostic = self.assertFails(
            self.check_with(x=table(rows)), "p1-address-above-255", file="x.ron", register="Operating Mode",
            field="address", found=300, allowed=(0, 255),
        )
        self.assertTrue(diagnostic.hint)

    # operating_modes
    def test_mode_row_without_operating_modes_fails(self) -> None:
        self.assertFails(
            self.check_with(x=table(operating_modes=None)), "operating-modes-missing", file="x.ron",
            field="operating_modes", register="Operating Mode",
        )
        self.assertFails(
            self.check(unmapped(extra=table(operating_modes=None))), "operating-modes-missing", file="extra.ron"
        )

    def test_operating_modes_without_mode_row_fails(self) -> None:
        bad = self.ax(operating_modes="(bounded: [3], unbounded: [], no_position: [])")
        self.assertFails(self.check_with(ax=bad), "operating-modes-without-mode-row", file="ax.ron", field="operating_modes")

    def test_value_in_two_lists_names_value_and_lists(self) -> None:
        modes = "(bounded: [3, 4], unbounded: [4, 5], no_position: [0, 1, 16])"
        self.assertFails(
            self.check_with(x=table(operating_modes=modes)), "mode-value-twice", file="x.ron", found=4,
            lists=["bounded", "unbounded"],
        )

    def test_malformed_operating_modes_fail(self) -> None:
        cases = (
            ("(bounded: [3], unbounded: [4, 5])", "operating-modes-key-missing", {"field": "operating_modes.no_position"}),
            ("(bounded: [3], unbounded: [4, 5], no_position: [0, 256])", "mode-value-invalid",
             {"field": "operating_modes.no_position", "found": 256, "allowed": (0, 255)}),
            ("(bounded: [3], unbounded: [4, 5], no_position: [-1])", "mode-value-invalid", {"found": -1}),
            ("(bounded: 3, unbounded: [4, 5], no_position: [0])", "mode-list-not-list",
             {"field": "operating_modes.bounded", "found": 3}),
            ("(bounded: [3], unbounded: [4, 5], no_position: [0], extra: [])", "operating-modes-unknown-key",
             {"field": "operating_modes", "found": ["extra"]}),
            ("[3, 4, 5]", "operating-modes-not-record", {"field": "operating_modes", "found": "a list"}),
        )
        for modes, cause, fields in cases:
            with self.subTest(modes=modes):
                self.assertFails(self.check_with(x=table(operating_modes=modes)), cause, file="x.ron", **fields)


class MapTests(FixtureCase):
    """Model map: US4 AS1–AS3, AS5, FR-008–FR-010, FR-012, FR-019, SC-007, xtask checks 2–6, 8, 13, 14, 16."""

    def run_map(self, entries: dict[int, str], tables: dict[str, str] | None = None, not_mapped: list[str] | None = None):
        for old in self.dir.glob("*.ron"):
            old.unlink()
        files = clean_files()
        for stem in ("x", "ax", "xl320"):
            files.pop(stem)
        files.update(tables or {"x": table()})
        files["dynamixel"] = model_map(entries, [] if not_mapped is None else not_mapped)
        return self.check(files)

    def run_raw_map(self, text: str) -> check_tables.Outcome:
        for old in self.dir.glob("*.ron"):
            old.unlink()
        return self.check({"x": table(), "dynamixel": text})

    def test_from_fw_order(self) -> None:
        ok = f'(family: "x-series", name: "X", p2: {ref_list((0, "x_fw52"), (53, "x"))})'
        files = {"x": table(protocols="[P2]"), "x_fw52": table(protocols="[P2]")}
        self.assertPasses(self.run_map({1190: ok}, files))
        first_one = f'(family: "x-series", name: "X", p2: {ref_list((1, "x"))})'
        self.assertFails(
            self.run_map({1190: first_one}, {"x": table(protocols="[P2]")}), "model-first-from-fw",
            file="dynamixel.ron", model=1190, field="p2", found=1, expected=0,
        )
        swapped = ref_list((0, "x"), (53, "x_fw52"), (40, "x2"))
        files = {"x": table(protocols="[P2]"), "x_fw52": table(protocols="[P2]"), "x2": table(protocols="[P2]")}
        self.assertFails(
            self.run_map({1190: f'(family: "x-series", name: "X", p2: {swapped})'}, files),
            "model-from-fw-order", model=1190, field="p2", found=[0, 53, 40],
        )

    def test_from_fw_range(self) -> None:
        refs = ref_list((0, "x"), (300, "x2"))
        files = {"x": table(protocols="[P2]"), "x2": table(protocols="[P2]")}
        self.assertFails(
            self.run_map({1190: f'(family: "x-series", name: "X", p2: {refs})'}, files),
            "model-from-fw-range", model=1190, field="p2", found=[300], allowed=(0, 255),
        )

    def test_missing_file(self) -> None:
        self.assertFails(
            self.run_map({1020: entry("x"), 1030: entry("gone")}), "model-file-missing", model=1030, table="gone"
        )

    def test_needs_a_non_empty_protocol_list(self) -> None:
        self.assertFails(self.run_map({1020: entry("x", ())}), "model-lists-missing", model=1020)
        empty = '(family: "x-series", name: "X", p1: [], p2: [(from_fw: 0, file: "x")], default: P2)'
        self.assertFails(self.run_map({1020: empty}), "model-list-empty", model=1020, field="p1")

    def test_protocol_lists_must_be_lists_of_refs(self) -> None:
        cases = (
            ('(family: "x-series", name: "X", p2: "x")', "model-list-not-list", {"field": "p2", "found": "x"}),
            ('(family: "x-series", name: "X", p2: [("x")])', "model-ref-invalid", {"field": "p2", "found": ["x"]}),
        )
        for text, cause, fields in cases:
            with self.subTest(text=text):
                self.assertFails(self.run_map({1020: text}, {"x": table(protocols="[P2]")}), cause, model=1020, **fields)

    def test_default_exactly_with_both_lists_and_p2(self) -> None:
        self.assertFails(
            self.run_map({1020: entry("x", default=None)}), "model-default-missing", model=1020, field="default"
        )
        self.assertFails(
            self.run_map({1020: entry("x", default="P1")}), "model-default-invalid", model=1020, field="default",
            found=Ident("P1"), expected="P2",
        )
        one = {"x": table(protocols="[P2]")}
        self.assertFails(
            self.run_map({1020: entry("x", ("p2",), default="P2")}, one), "model-default-unexpected", model=1020,
            field="default", found=Ident("P2"),
        )

    def test_family(self) -> None:
        self.assertFails(
            self.run_map({1020: entry("x", family="dxl")}), "model-family-invalid", model=1020, field="family",
            found="dxl", allowed=check_tables.FAMILIES,
        )
        for family in ("ax", "mx", "rx", "x-series", "y-series", "pro"):
            with self.subTest(family=family):
                self.assertPasses(self.run_map({1020: entry("x", family=family)}))

    def test_name(self) -> None:
        for text, found in (('(family: "x-series", name: "", p2: [(from_fw: 0, file: "x")])', ""),
                            ('(family: "x-series", p2: [(from_fw: 0, file: "x")])', None)):
            with self.subTest(text=text):
                self.assertFails(
                    self.run_map({1020: text}, {"x": table(protocols="[P2]")}), "model-name-invalid", model=1020,
                    field="name", found=found,
                )

    def test_file_under_one_number_only(self) -> None:
        self.assertFails(
            self.run_map({1020: entry("x"), 1030: entry("x")}), "file-under-two-numbers", table="x",
            found=[1020, 1030],
        )
        self.assertPasses(self.run_map({1020: entry("x")}))

    def test_list_needs_the_protocol_in_the_table(self) -> None:
        self.assertFails(
            self.run_map({1020: entry("x", ("p1",))}, {"x": table(protocols="[P2]")}), "model-file-protocol",
            model=1020, field="p1", table="x", found=["P2"], expected="P1",
        )
        self.assertFails(
            self.run_map({1020: entry("x", ("p2",))}, {"x": table(protocols="[P1]")}), "model-file-protocol",
            model=1020, field="p2", table="x", found=["P1"], expected="P2",
        )

    def test_dual_protocol_table_needs_both_lists(self) -> None:
        self.assertFails(self.run_map({1020: entry("x", ("p2",))}), "model-needs-both-lists", model=1020, table="x")

    def test_tables_key_is_rejected(self) -> None:
        # ron-schema §2: "Absent after T2: from T2 on, the check rejects a `tables` key in any model entry".
        diagnostic = self.assertFails(
            self.run_map({1020: entry("x", tables=ref_list("x"))}), "model-legacy-tables", model=1020, field="tables"
        )
        self.assertTrue(diagnostic.hint)
        self.assertNotIn("p1", diagnostic.message)
        p1_only = {"ax": table(AX_ROWS, protocols="[P1]", type_info=AX_SCALE)}
        self.assertFails(
            self.run_map({12: entry("ax", ("p1",), family="ax", tables=ref_list("ax"))}, p1_only),
            "model-legacy-tables", model=12,
        )
        self.assertFails(self.run_map({1020: entry("x", tables="[]")}), "model-legacy-tables", model=1020)
        self.assertPasses(self.run_map({1020: entry("x")}))

    def test_also_is_a_non_empty_list_of_names(self) -> None:
        # data-model "`also` | list of string | no | other products sold under this number (docs only)"; US5 AS1.
        ax = {"ax": table(AX_ROWS, protocols="[P1]", type_info=AX_SCALE)}
        p1 = ref_list("ax")

        def ax_entry(also: str | None) -> str:
            extra = "" if also is None else f", also: {also}"
            return f'(family: "ax", name: "AX-12A"{extra}, p1: {p1})'

        self.assertPasses(self.run_map({12: ax_entry('["AX-12+"]')}, ax))
        self.assertPasses(self.run_map({12: ax_entry('["AX-12+", "AX-12"]')}, ax))
        self.assertPasses(self.run_map({12: ax_entry(None)}, ax))  # a P1-only entry without `default`
        cases = (
            ('"AX-12+"', "model-also-shape", "AX-12+"),
            ("[]", "model-also-shape", []),
            ('[""]', "model-also-names", [""]),
            ("[12]", "model-also-names", [12]),
            ('["AX-12+", "AX-12+"]', "model-also-names", ["AX-12+", "AX-12+"]),
        )
        for bad, cause, found in cases:
            with self.subTest(also=bad):
                self.assertFails(self.run_map({12: ax_entry(bad)}, ax), cause, model=12, field="also", found=found)

    def test_firmware_version_at_one_place_per_list(self) -> None:
        moved = replaced(X_ROWS, "Firmware Version", address=7)
        files = {"x": table(protocols="[P2]"), "x_fw52": table(moved, protocols="[P2]")}
        refs = ref_list((0, "x_fw52"), (53, "x"))
        self.assertFails(
            self.run_map({1190: f'(family: "x-series", name: "X", p2: {refs})'}, files),
            "model-firmware-version-places", model=1190, field="p2", register="Firmware Version",
            found=[(6, 1), (7, 1)],
        )

    def test_p1_role_above_255(self) -> None:
        rows = replaced(X_ROWS, "Goal Position", address=300)
        self.assertFails(
            self.run_map({1020: entry("x")}, {"x": table(rows)}), "p1-address-above-255", file="x.ron",
            register="Goal Position", found=300,
        )
        self.assertPasses(self.run_map({1020: entry("x", ("p2",))}, {"x": table(rows, protocols="[P2]")}))

    def test_p1_hardware_error_status_above_255(self) -> None:
        # xtask check 14 (X1, resolve.rs `check_p1_addresses`) counts the optional fault register as a role.
        rows = X_ROWS + [(300, 1, "Hardware Error Status", "R", "RAM")]
        self.assertFails(
            self.run_map({1020: entry("x")}, {"x": table(rows)}), "p1-address-above-255", file="x.ron",
            register="Hardware Error Status", found=300,
        )
        self.assertPasses(self.run_map({1020: entry("x", ("p2",))}, {"x": table(rows, protocols="[P2]")}))
        fine = X_ROWS + [(70, 1, "Hardware Error Status", "R", "RAM")]
        self.assertPasses(self.run_map({1020: entry("x")}, {"x": table(fine)}))

    def test_type_and_module_identifiers(self) -> None:
        files = {"ax_12a": table(), "ax12a": table()}
        self.assertFails(
            self.run_map({1: entry("ax_12a"), 2: entry("ax12a")}, files), "type-ident-duplicate", table="ax_12a",
            other="ax12a", found="Ax12a",
        )
        self.assertFails(self.run_map({1: entry("tests")}, {"tests": table()}), "module-ident-reserved", table="tests", found="tests")
        self.assertFails(self.run_map({1: entry("type")}, {"type": table()}), "module-ident-reserved", table="type", found="type")
        self.assertPasses(self.run_map({1090: entry("2xl430_w250")}, {"2xl430_w250": table()}))

    def test_map_shape(self) -> None:
        cases = (
            ("[1]", "map-not-record", {"found": "a list"}),
            ("(models: [1])", "models-not-map", {"field": "models", "found": "a list"}),
            ('(models: { 70000: (family: "x-series", name: "X", p2: [(from_fw: 0, file: "x")]) })',
             "model-number-invalid", {"model": 70000, "found": 70000, "allowed": (0, 65535)}),
            ('(models: { 1020: "x" })', "model-entry-not-record", {"model": 1020, "found": "string 'x'"}),
        )
        for text, cause, fields in cases:
            with self.subTest(text=text):
                self.assertFails(self.run_raw_map(text), cause, file="dynamixel.ron", **fields)

    def test_dual_protocol_section(self) -> None:
        code, out, _err, _diagnostics = self.check(clean_files())
        self.assertEqual(code, 0, out)
        section = out.split("dual-protocol models\n", 1)[1].split("errors\n", 1)[0]
        self.assertEqual(section.strip(), "1020 XM430-W350   p1: x   p2: x   default: P2")
        self.assertRegex(out, r"\n  x\s+unverified\s+\[P1, P2\]\s+1020 p1 p2\n")
        self.assertRegex(out, r"\n  ax\s+unverified\s+\[P1\]\s+12 p1\n")


class NotMappedTests(FixtureCase):
    """`not_mapped`: US5 AS4–AS5, FR-011, SC-003, xtask checks 6–7."""

    ENTRIES = {12: entry("ax", ("p1",), family="ax"), 350: entry("xl320", ("p2",)), 1020: entry("x", name="XM430-W350")}

    def run_nm(self, records: list[str] | None, entries: dict[int, str] | None = None, **tables: str):
        files = clean_files()
        files.update(tables)
        files["dynamixel"] = model_map(entries or self.ENTRIES, records)
        return self.check(files)

    def test_unrecorded_table_fails(self) -> None:
        for records in ([], None):
            with self.subTest(records=records):
                self.assertFails(self.run_nm(records, extra=table()), "table-unlisted", file="extra.ron", table="extra")

    def test_recorded_table_passes_and_shows_its_reason(self) -> None:
        result = self.run_nm(['(file: "extra", reason: SameNumberAs(12))', '(file: "other", reason: NoModelNumber)'],
                             extra=table(), other=table())
        self.assertPasses(result)
        self.assertRegex(result.out, r"\n  extra\s+unverified\s+\[P1, P2\]\s+not mapped: same number as 12\n")
        self.assertRegex(result.out, r"\n  other\s+unverified\s+\[P1, P2\]\s+not mapped: no model number\n")
        self.assertIn("2 not mapped", result.out)

    def test_mapped_and_recorded_fails(self) -> None:
        self.assertFails(
            self.run_nm(['(file: "x", reason: SameNumberAs(12))']), "not-mapped-also-mapped", file="dynamixel.ron",
            table="x", models=[1020],
        )

    def test_record_for_missing_file_fails(self) -> None:
        self.assertFails(self.run_nm(['(file: "gone", reason: NoModelNumber)']), "not-mapped-file-missing", table="gone")

    def test_same_number_as_unknown_number_fails(self) -> None:
        self.assertFails(
            self.run_nm(['(file: "extra", reason: SameNumberAs(99))'], extra=table()), "not-mapped-number-unknown",
            table="extra", model=99,
        )

    def test_other_reasons_fail(self) -> None:
        for reason, found in (
            ("Unverified", Ident("Unverified")),
            ("SameNumberAs(70000)", Tagged("SameNumberAs", [70000])),
            ("SameNumberAs()", Tagged("SameNumberAs", {})),
            ('"NoModelNumber"', "NoModelNumber"),
            ("SameNumberAs(12, 13)", Tagged("SameNumberAs", [12, 13])),
        ):
            with self.subTest(reason=reason):
                diagnostic = self.assertFails(
                    self.run_nm([f'(file: "extra", reason: {reason})'], extra=table()), "not-mapped-reason-invalid",
                    table="extra", field="reason", found=found,
                )
                self.assertTrue(diagnostic.hint)

    def test_record_listed_twice_fails(self) -> None:
        records = ['(file: "extra", reason: NoModelNumber)', '(file: "extra", reason: NoModelNumber)']
        self.assertFails(self.run_nm(records, extra=table()), "not-mapped-duplicate", table="extra")

    def test_not_mapped_shape(self) -> None:
        text = model_map(self.ENTRIES).replace(")\n", "  not_mapped: 3,\n)\n")
        self.assertFails(self.check({**clean_files(), "dynamixel": text}), "not-mapped-not-list", field="not_mapped", found="integer 3")
        self.assertFails(
            self.run_nm(['"extra"'], extra=table()), "not-mapped-record-invalid", index=0, found="string 'extra'"
        )


UNIT_SOURCE = 'unit_source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/", verified: "2026-10-28"),'
FR9_X_ROWS = X_ROWS[:2] + [
    (10, 1, "Drive Mode", "RW", "EEPROM"),
    (11, 1, "Operating Mode", "RW", "EEPROM"),
    (38, 2, "Current Limit", "RW", "EEPROM"),
    (44, 4, "Velocity Limit", "RW", "EEPROM"),
] + X_ROWS[3:] + [
    (84, 2, "Position P Gain", "RW", "RAM", (0, 16383)),
    (102, 2, "Goal Current", "RW", "RAM"),
    (104, 4, "Goal Velocity", "RW", "RAM"),
    (112, 4, "Profile Velocity", "RW", "RAM"),
    (128, 4, "Present Velocity", "R", "RAM"),
]
FR9_X_UNITS = """  unit_info: [
    { data_name: "Current Limit", value: 2.69, unit: "mA", sign_type: "unsigned", range: (min: 0, max: 1193) },
    { data_name: "Goal Current", value: 2.69, unit: "mA", sign_type: "twos_complement", bounded_by: "Current Limit" },
    { data_name: "Velocity Limit", value: 0.229, unit: "rpm", sign_type: "unsigned", range: (min: 0, max: 1023) },
    { data_name: "Goal Velocity", value: 0.229, unit: "rpm", sign_type: "twos_complement", bounded_by: "Velocity Limit" },
    { data_name: "Profile Velocity", value: 0.229, unit: "rpm", sign_type: "unsigned", profile: "velocity_based",
      zero_is_unlimited: true, range: (min: 0, max: 32767) },
    { data_name: "Profile Velocity", value: 1.0, unit: "ms", sign_type: "unsigned", profile: "time_based",
      zero_is_unlimited: true, range: (min: 0, max: 32737), from_fw: 42 },
    { data_name: "Present Velocity", value: 0.229, unit: "rpm", sign_type: "twos_complement" },
  ],"""
FR9_X_MODES = (
    "(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], "
    "current: [0], velocity: [1], pwm: [16], current_position: [5])"
)
FR9_AX_ROWS = AX_ROWS[:5] + [
    (28, 1, "CW Compliance Slope", "RW", "RAM", (0, 254)),
    (30, 2, "Goal Position", "RW", "RAM"),
    (32, 2, "Moving Speed", "RW", "RAM"),
    (36, 2, "Present Position", "R", "RAM"),
    (40, 2, "Present Load", "R", "RAM"),
    (70, 1, "Torque Ctrl Mode Enable", "RW", "RAM"),
    (71, 2, "Goal Torque", "RW", "RAM"),
]
FR9_AX_UNITS = """  unit_info: [
    { data_name: "Moving Speed", value: 0.114, unit: "rpm", sign_type: "unsigned", modes: ["joint", "multi_turn"],
      zero_is_unlimited: true, range: (min: 0, max: 1023) },
    { data_name: "Moving Speed", value: 0.114, unit: "rpm", sign_type: "direction_bit_10", modes: ["wheel"],
      range: (min: 0, max: 2047) },
    { data_name: "Present Load", value: 0.1, unit: "%", sign_type: "direction_bit_10", range: (min: 0, max: 2047) },
    { data_name: "Goal Torque", value: 4.5, unit: "mA", sign_type: "direction_bit_10", modes: ["torque_control"] },
  ],"""
TORQUE_MODE = 'torque_control_mode: (register: "Torque Ctrl Mode Enable", on: 1, goal: "Goal Torque"),'


def fr9_x(*, units: str = FR9_X_UNITS, source: str | None = UNIT_SOURCE, rows: list[tuple] | None = None,
          modes: str = FR9_X_MODES, more: str = "") -> str:
    """An X table carrying FR-9 data (`unit_source` unless `source` is None)."""
    extra = "\n".join(part for part in (f"  {source}" if source else "", units, more) if part)
    return table(FR9_X_ROWS if rows is None else rows, operating_modes=modes, extra=extra)


def fr9_ax(*, units: str = FR9_AX_UNITS, source: str | None = UNIT_SOURCE, torque: str | None = TORQUE_MODE,
           rows: list[tuple] | None = None) -> str:
    """An AX-shaped (angle-limit) table with MX Protocol 1 style FR-9 data."""
    extra = "\n".join(part for part in (f"  {source}" if source else "", f"  {torque}" if torque else "", units) if part)
    return table(FR9_AX_ROWS if rows is None else rows, protocols="[P1]", type_info=AX_SCALE, extra=extra)


def units(*entries: str) -> str:
    return "  unit_info: [\n" + "".join(f"    {e},\n" for e in entries) + "  ],"


class Fr9Tests(FixtureCase):
    """FR-9 register data (specs/009-tables-fr9): `unit_source`, `unit_info`, ranges, per-mode lists."""

    def test_clean_fr9_tables_pass(self) -> None:
        self.assertPasses(self.check_with(x=fr9_x(), ax=fr9_ax()))

    def test_legacy_unit_info_passes_without_unit_source(self) -> None:
        legacy = units(
            '{ data_name: "Speed", value: 0.111, unit: "rpm", sign_type: "signed" }',
            '{ data_name: "Torque", value: 0.001757, unit: "N·m/bit", sign_type: "unsigned" }',
        )
        self.assertPasses(self.check_with(x=table(extra=legacy)))

    def test_legacy_unit_info_shape_is_checked(self) -> None:
        cases = (
            ('{ data_name: "Speed", value: 0.1, unit: "rpm", sign_type: "sideways" }', "unit-entry-sign-unknown",
             {"register": "Speed", "field": "sign_type", "found": "sideways"}),
            ('{ data_name: "Speed", value: 0, unit: "rpm", sign_type: "signed" }', "unit-entry-value-invalid",
             {"register": "Speed", "field": "value", "found": 0}),
            ('{ data_name: "Speed", value: 0.1, sign_type: "signed" }', "unit-entry-key-missing",
             {"register": "Speed", "field": "unit"}),
            ('{ data_name: "", value: 0.1, unit: "rpm", sign_type: "signed" }', "unit-entry-name-invalid",
             {"field": "data_name", "found": ""}),
            ('{ data_name: "Speed", value: 0.1, unit: "rpm", sign_type: "signed", colour: 1 }',
             "unit-entry-unknown-field", {"register": "Speed", "found": ["colour"]}),
            ('{ data_name: "Speed", value: 0.1, unit: "", sign_type: "signed" }', "unit-entry-unit-invalid",
             {"register": "Speed", "field": "unit", "found": ""}),
            ('"Speed"', "unit-entry-not-record", {"found": "string 'Speed'"}),
        )
        for text, cause, fields in cases:
            with self.subTest(entry=text):
                self.assertFails(self.check_with(x=table(extra=units(text))), cause, file="x.ron", index=0, **fields)
        self.assertFails(
            self.check_with(x=table(extra="  unit_info: 3,")), "unit-info-not-list", file="x.ron", field="unit_info",
            found="integer 3",
        )

    def test_fr9_fields_need_unit_source(self) -> None:
        result = self.check_with(x=fr9_x(source=None))
        diagnostic = self.assertFails(
            result, "unit-entry-fr9-needs-unit-source", file="x.ron", register="Current Limit", found=["range"]
        )
        self.assertTrue(diagnostic.hint)
        self.assertFails(
            result, "per-mode-lists-need-unit-source", field="operating_modes",
            found=["current", "velocity", "pwm", "current_position"],
        )
        self.assertFails(
            self.check_with(ax=fr9_ax(source=None)), "torque-control-needs-unit-source", file="ax.ron",
            field="torque_control_mode",
        )
        ranged = [r if r[2] != "Goal Position" else (*r, (0, 4095)) for r in X_ROWS]
        self.assertFails(
            self.check_with(x=table(ranged)), "row-range-needs-unit-source", file="x.ron", register="Goal Position",
            field="range",
        )

    def test_unit_source_must_be_a_verified_emanual_page(self) -> None:
        cases = (
            ("unit_source: Unverified,", "source-shape", {"field": "unit_source", "found": Ident("Unverified")}),
            ('unit_source: Verified(url: "https://example.com/x", verified: "2026-10-28"),', "url-not-emanual",
             {"field": "unit_source.url", "found": "https://example.com/x"}),
            ('unit_source: Verified(url: "https://emanual.robotis.com/x", verified: "2026-12-01"),',
             "source-date-future", {"field": "unit_source.verified", "found": "2026-12-01"}),
            ('unit_source: Verified(url: "https://emanual.robotis.com/x"),', "source-date-missing",
             {"field": "unit_source.verified"}),
        )
        for text, cause, fields in cases:
            with self.subTest(source=text):
                self.assertFails(self.check_with(x=fr9_x(source=text)), cause, file="x.ron", **fields)

    def test_strict_entry_rules(self) -> None:
        cases = (
            ('{ data_name: "Goal Speed", value: 0.229, unit: "rpm", sign_type: "unsigned" }', "unit-entry-no-row",
             {"register": "Goal Speed"}),
            ('{ data_name: "Present Velocity", value: 0.229, unit: "rev/min", sign_type: "twos_complement" }',
             "unit-entry-unit-unknown", {"register": "Present Velocity", "found": "rev/min", "allowed": check_tables.UNITS}),
            ('{ data_name: "Present Velocity", value: 0.229, unit: "rpm", sign_type: "signed" }',
             "unit-entry-sign-invalid", {"register": "Present Velocity", "field": "sign_type", "found": "signed"}),
            ('{ data_name: "Goal Current", value: 2.69, unit: "mA", sign_type: "direction_bit_16" }',
             "unit-entry-direction-bit-outside", {"register": "Goal Current", "found": 16, "size": 2, "allowed": (1, 15)}),
            ('{ data_name: "Goal Current", value: 2.69, unit: "mA", sign_type: "direction_bit_0" }',
             "unit-entry-direction-bit-outside", {"found": 0, "size": 2}),
            ('{ data_name: "Goal Current", value: -1, unit: "mA", sign_type: "unsigned" }', "unit-entry-value-invalid",
             {"register": "Goal Current", "found": -1}),
            ('{ data_name: "Goal Current", value: 2.69, unit: "mA", sign_type: "unsigned", scale: 2 }',
             "unit-entry-unknown-field", {"found": ["scale"]}),
            ('{ data_name: "Goal Current", value: 2.69, unit: "mA", sign_type: "unsigned", from_fw: 300 }',
             "unit-entry-from-fw-invalid", {"field": "from_fw", "found": 300, "allowed": (0, 255)}),
            ('{ data_name: "Goal Current", value: 2.69, unit: "mA", sign_type: "unsigned", zero_is_unlimited: 1 }',
             "unit-entry-zero-is-unlimited-invalid", {"field": "zero_is_unlimited", "found": 1}),
        )
        for text, cause, fields in cases:
            with self.subTest(entry=text):
                self.assertFails(self.check_with(x=fr9_x(units=units(text))), cause, file="x.ron", index=0, **fields)

    def test_ranges_fit_the_register(self) -> None:
        cases = (
            ("(min: 10, max: 2)", "range-min-above-max", {"min": 10, "max": 2}),
            ("(min: 0, max: 65536)", "range-does-not-fit",
             {"found": (0, 65536), "allowed": (0, 65535), "size": 2, "sign_type": "unsigned"}),
            ("(min: -1, max: 2)", "range-does-not-fit", {"found": (-1, 2)}),
            ("(min: 0)", "range-shape", {"found": {"min": 0}}),
            ('(min: 0, max: "x")', "range-not-int", {"found": (0, "x")}),
        )
        for span, cause, fields in cases:
            with self.subTest(span=span):
                entry = f'{{ data_name: "Current Limit", value: 2.69, unit: "mA", sign_type: "unsigned", range: {span} }}'
                self.assertFails(
                    self.check_with(x=fr9_x(units=units(entry))), cause, file="x.ron", register="Current Limit",
                    field="range", **fields,
                )
        twos = '{ data_name: "Goal Current", value: 2.69, unit: "mA", sign_type: "twos_complement", range: (min: -1193, max: 1193) }'
        self.assertPasses(self.check_with(x=fr9_x(units=units(twos))))
        bit = '{ data_name: "Goal Current", value: 2.69, unit: "mA", sign_type: "direction_bit_10", range: (min: 0, max: 2048) }'
        self.assertFails(
            self.check_with(x=fr9_x(units=units(bit))), "range-does-not-fit", register="Goal Current",
            found=(0, 2048), allowed=(0, 2047), sign_type="direction_bit_10",
        )

    def test_row_ranges_fit_the_row(self) -> None:
        for span, cause, fields in (
            ((0, 70000), "range-does-not-fit", {"found": (0, 70000), "size": 2}),
            ((-40000, 0), "range-does-not-fit", {"found": (-40000, 0)}),
            ((5, 1), "range-min-above-max", {"min": 5, "max": 1}),
        ):
            with self.subTest(span=span):
                rows = [r if r[2] != "Position P Gain" else (*r[:5], span) for r in FR9_X_ROWS]
                self.assertFails(
                    self.check_with(x=fr9_x(rows=rows)), cause, file="x.ron", register="Position P Gain", field="range",
                    **fields,
                )
        rows = [r if r[2] != "Position P Gain" else (*r[:5], (-32768, 32767)) for r in FR9_X_ROWS]
        self.assertPasses(self.check_with(x=fr9_x(rows=rows)))

    def test_modes_only_in_angle_limit_tables(self) -> None:
        entry = '{ data_name: "Goal Velocity", value: 0.229, unit: "rpm", sign_type: "twos_complement", modes: ["wheel"] }'
        self.assertFails(
            self.check_with(x=fr9_x(units=units(entry))), "unit-entry-modes-need-angle-limits", file="x.ron",
            register="Goal Velocity", field="modes",
        )

    def test_mode_names(self) -> None:
        cases = (
            ('["spin"]', "unit-entry-mode-unknown", {"found": "spin", "allowed": check_tables.MODES_P1}),
            ("[]", "unit-entry-modes-invalid", {"found": []}),
            ('["wheel", "wheel"]', "unit-entry-mode-duplicate", {"found": "wheel"}),
        )
        for modes, cause, fields in cases:
            with self.subTest(modes=modes):
                entry = f'{{ data_name: "Present Load", value: 0.1, unit: "%", sign_type: "direction_bit_10", modes: {modes} }}'
                self.assertFails(
                    self.check_with(ax=fr9_ax(units=units(entry))), cause, file="ax.ron", register="Present Load",
                    field="modes", **fields,
                )
        self.assertFails(
            self.check_with(ax=fr9_ax(torque=None)), "unit-entry-mode-needs-torque-control", file="ax.ron",
            register="Goal Torque", found="torque_control",
        )

    def test_profile_needs_a_drive_mode_row(self) -> None:
        rows = [r for r in FR9_X_ROWS if r[2] != "Drive Mode"]
        self.assertFails(
            self.check_with(x=fr9_x(rows=rows)), "unit-entry-profile-needs-drive-mode", file="x.ron",
            register="Profile Velocity", field="profile",
        )
        entry = '{ data_name: "Profile Velocity", value: 0.229, unit: "rpm", sign_type: "unsigned", profile: "smooth" }'
        self.assertFails(
            self.check_with(x=fr9_x(units=units(entry))), "unit-entry-profile-invalid", register="Profile Velocity",
            field="profile", found="smooth", allowed=check_tables.PROFILES,
        )

    def test_one_entry_per_register_mode_and_profile(self) -> None:
        twice = '{ data_name: "Present Velocity", value: 0.229, unit: "rpm", sign_type: "twos_complement" }'
        diagnostic = self.assertFails(
            self.check_with(x=fr9_x(units=units(twice, twice))), "unit-entry-overlap", file="x.ron",
            register="Present Velocity", index=1, other=0,
        )
        self.assertTrue(diagnostic.hint)
        speed = '{{ data_name: "Moving Speed", value: 0.114, unit: "rpm", sign_type: "unsigned", modes: {} }}'
        overlap = units(speed.format('["joint", "wheel"]'), speed.format('["wheel"]'))
        self.assertFails(
            self.check_with(ax=fr9_ax(units=overlap)), "unit-entry-overlap", register="Moving Speed", index=1, other=0,
            mode="wheel",
        )
        apart = units(speed.format('["joint"]'), speed.format('["wheel"]'), speed.format('["multi_turn"]'))
        self.assertPasses(self.check_with(ax=fr9_ax(units=apart)))

    def test_an_entry_without_modes_or_profile_overlaps_every_one(self) -> None:
        every = '{ data_name: "Moving Speed", value: 0.114, unit: "rpm", sign_type: "unsigned" }'
        wheel = '{ data_name: "Moving Speed", value: 0.114, unit: "rpm", sign_type: "direction_bit_10", modes: ["wheel"] }'
        for pair in ((every, wheel), (wheel, every)):
            with self.subTest(first=pair[0]):
                self.assertFails(
                    self.check_with(ax=fr9_ax(units=units(*pair))), "unit-entry-overlap", register="Moving Speed",
                    index=1, other=0,
                )
        both = '{ data_name: "Profile Velocity", value: 0.229, unit: "rpm", sign_type: "unsigned" }'
        timed = '{ data_name: "Profile Velocity", value: 1.0, unit: "ms", sign_type: "unsigned", profile: "time_based" }'
        self.assertFails(
            self.check_with(x=fr9_x(units=units(both, timed))), "unit-entry-overlap", register="Profile Velocity",
            index=1, other=0, profile="time_based",
        )

    def test_bounded_by(self) -> None:
        limit = '{ data_name: "Current Limit", value: 2.69, unit: "mA", sign_type: "unsigned" }'
        goal = '{{ data_name: "Goal Current", value: {v}, unit: "mA", sign_type: "twos_complement", bounded_by: {b} }}'
        cases = (
            (goal.format(v=2.69, b='"Current Cap"'), "bounded-by-no-row", {"found": "Current Cap"}),
            (goal.format(v=2.69, b='"Goal Current"'), "bounded-by-self", {"found": "Goal Current"}),
            (goal.format(v=3.36, b='"Current Limit"'), "bounded-by-scale-mismatch",
             {"found": "Current Limit", "expected": (3.36, "mA")}),
            (goal.format(v=2.69, b="3"), "bounded-by-no-row", {"found": 3}),
        )
        for text, cause, fields in cases:
            with self.subTest(entry=text):
                self.assertFails(
                    self.check_with(x=fr9_x(units=units(limit, text))), cause, file="x.ron", register="Goal Current",
                    field="bounded_by", index=1, **fields,
                )
        no_limit_entry = goal.format(v=2.69, b='"Current Limit"')
        self.assertFails(
            self.check_with(x=fr9_x(units=units(no_limit_entry))), "bounded-by-scale-mismatch", index=0,
            found="Current Limit",
        )

    def test_per_mode_lists(self) -> None:
        cases = (
            ("(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: [0], velocity: [1], pwm: [16])",
             "per-mode-list-missing", {"field": "operating_modes.current_position"}),
            ("(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: [0], velocity: [1], pwm: [], "
             "current_position: [5])", "per-mode-value-uncovered", {"field": "operating_modes.no_position", "found": [16]}),
            ("(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: [0, 1], velocity: [1], pwm: [16], "
             "current_position: [5])", "per-mode-value-twice", {"found": 1, "lists": ["current", "velocity"]}),
            ("(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: [0], velocity: [1], pwm: [16], "
             "current_position: [3])", "per-mode-value-outside-parent",
             {"field": "operating_modes.current_position", "found": [3], "parent": "unbounded"}),
            ("(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: [0], velocity: [1, 2], pwm: [16], "
             "current_position: [5])", "per-mode-value-outside-parent",
             {"field": "operating_modes.velocity", "found": [2], "parent": "no_position"}),
            ("(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: 0, velocity: [1], pwm: [16], "
             "current_position: [5])", "mode-list-not-list", {"field": "operating_modes.current", "found": 0}),
            ("(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: [0], velocity: [1], pwm: [999], "
             "current_position: [5])", "mode-value-invalid", {"field": "operating_modes.pwm", "found": 999}),
        )
        for modes, cause, fields in cases:
            with self.subTest(modes=modes):
                self.assertFails(self.check_with(x=fr9_x(modes=modes)), cause, file="x.ron", **fields)
        empty = "(bounded: [3], unbounded: [], no_position: [], current: [], velocity: [], pwm: [], current_position: [])"
        self.assertPasses(self.check_with(x=fr9_x(modes=empty, units="")))

    def test_torque_control_mode(self) -> None:
        cases = (
            ('(register: "Torque Mode", on: 1, goal: "Goal Torque")', "torque-control-no-row",
             {"field": "torque_control_mode.register", "found": "Torque Mode"}),
            ('(register: "Torque Ctrl Mode Enable", on: 1, goal: "Goal Push")', "torque-control-no-row",
             {"field": "torque_control_mode.goal", "found": "Goal Push"}),
            ('(register: "Goal Torque", on: 1, goal: "Goal Torque")', "torque-control-row-attributes",
             {"field": "torque_control_mode.register", "register": "Goal Torque", "found": (2, "RW", "RAM"),
              "expected": (1, "RW", "RAM")}),
            ('(register: "Torque Ctrl Mode Enable", on: 1, goal: "Present Load")', "torque-control-row-attributes",
             {"field": "torque_control_mode.goal", "register": "Present Load", "found": (2, "R", "RAM"),
              "expected": (None, "RW", "RAM")}),
            ('(register: "Torque Ctrl Mode Enable", on: 256, goal: "Goal Torque")', "torque-control-on-invalid",
             {"field": "torque_control_mode.on", "found": 256, "allowed": (0, 255)}),
            ('(register: "Torque Ctrl Mode Enable", goal: "Goal Torque")', "torque-control-key-missing",
             {"field": "torque_control_mode.on"}),
            ('(register: "Torque Ctrl Mode Enable", on: 1, goal: "Goal Torque", off: 0)', "torque-control-unknown-key",
             {"field": "torque_control_mode", "found": ["off"]}),
            ('(register: "Torque Ctrl Mode Enable", on: 1, goal: "Torque Ctrl Mode Enable")', "torque-control-same-row",
             {"register": "Torque Ctrl Mode Enable"}),
            ("[1]", "torque-control-not-record", {"field": "torque_control_mode", "found": "a list"}),
        )
        for text, cause, fields in cases:
            with self.subTest(spec=text):
                self.assertFails(
                    self.check_with(ax=fr9_ax(torque=f"torque_control_mode: {text},")), cause, file="ax.ron", **fields
                )
        rows = FR9_X_ROWS + [(70, 1, "Torque Ctrl Mode Enable", "RW", "RAM"), (71, 2, "Goal Torque", "RW", "RAM")]
        on_x = fr9_x(rows=rows, more=f"  {TORQUE_MODE}")
        self.assertFails(
            self.check_with(x=on_x), "torque-control-needs-angle-limits", file="x.ron", field="torque_control_mode"
        )

    def test_torque_control_on_value_fits_the_register_range(self) -> None:
        rows = [r if r[2] != "Torque Ctrl Mode Enable" else (*r, (0, 1)) for r in FR9_AX_ROWS]
        self.assertPasses(self.check_with(ax=fr9_ax(rows=rows)))
        spec = 'torque_control_mode: (register: "Torque Ctrl Mode Enable", on: 2, goal: "Goal Torque"),'
        self.assertFails(
            self.check_with(ax=fr9_ax(rows=rows, torque=spec)), "torque-control-on-outside-range", file="ax.ron",
            field="torque_control_mode.on", register="Torque Ctrl Mode Enable", found=2, allowed=(0, 1),
        )

    def test_torque_control_rows_stay_in_protocol_1_addresses(self) -> None:
        rows = [r if r[2] != "Goal Torque" else (300, 2, "Goal Torque", "RW", "RAM") for r in FR9_AX_ROWS]
        self.assertFails(
            self.check_with(ax=fr9_ax(rows=rows)), "p1-address-above-255", file="ax.ron",
            field="torque_control_mode.goal", register="Goal Torque", found=300, allowed=(0, 255),
        )


class RealTablesTests(unittest.TestCase):
    """The FR-9 data in this repository's own tables (specs/009-tables-fr9)."""

    FR9_STEMS = ("ax_12a", "ax_12_plus", "rx_28", "mx_64_p1", "mx_64", "xm430_w350")

    def test_repository_tables_pass(self) -> None:
        result = run_check(["--today", "2026-11-03"])
        self.assertEqual(result.code, 0, result.err)
        self.assertEqual(result.diagnostics, [])

    def test_fr9_tables_carry_unit_source(self) -> None:
        _map, tables = check_tables.load(Path(check_tables.__file__).resolve().parent)
        for stem in self.FR9_STEMS:
            with self.subTest(stem=stem):
                self.assertIsInstance(tables[stem]["unit_source"], Tagged)
                names = {e["data_name"] for e in tables[stem]["unit_info"]}
                self.assertIn("Present Temperature", names)
        self.assertEqual(
            tables["mx_64"]["operating_modes"]["current_position"], [5]
        )
        self.assertEqual(tables["mx_64_p1"]["torque_control_mode"]["register"], "Torque Ctrl Mode Enable")

    def test_mx_64_p1_positions_do_not_claim_multi_turn(self) -> None:
        _map, tables = check_tables.load(Path(check_tables.__file__).resolve().parent)
        for entry in tables["mx_64_p1"]["unit_info"]:
            if entry["data_name"] in ("Goal Position", "Present Position"):
                with self.subTest(register=entry["data_name"]):
                    self.assertNotIn("multi_turn", entry["modes"])
                    self.assertEqual(entry["range"], {"min": 0, "max": 4095})


if __name__ == "__main__":
    unittest.main()

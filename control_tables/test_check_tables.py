"""Tests for `check_tables.py` (specs/005-tables/contracts/table-tools.md section 1).

Run with `python3 -m unittest discover -s control_tables -p 'test_*.py'`. No network.
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


def run_check(argv: list[str]) -> tuple[int, str, str]:
    """Calls `check_tables.main(argv)` and returns (exit code, stdout, stderr)."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = check_tables.main(argv)
    return code, out.getvalue(), err.getvalue()


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


def row(address: int, size: int, name: str, access: str | None = "RW", area: str | None = "RAM") -> str:
    fields = [f"address: {address}", f"size: {size}", f'data_name: "{name}"']
    if access is not None:
        fields.append(f"access: {access}")
    if area is not None:
        fields.append(f"area: {area}")
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
    tables: str | None = AUTO,  # type: ignore[assignment]
) -> str:
    """One model entry mapping `stem` under `lists`, with the legacy `tables` list."""
    ref = ref_list(stem)
    parts = [f'family: "{family}"', f'name: "{name or stem.upper()}"']
    parts += [f"{key}: {ref}" for key in lists]
    if default is AUTO:
        default = "P2" if len(lists) == 2 else None
    if default is not None:
        parts.append(f"default: {default}")
    if tables is AUTO:
        tables = ref
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

    def check(self, files: dict[str, str], *extra: str) -> tuple[int, str, str]:
        write_fixture(self.dir, files)
        return run_check(["--dir", str(self.dir), "--today", TODAY, *extra])

    def check_with(self, *extra: str, **changes: str | None) -> tuple[int, str, str]:
        """Runs the check on `clean_files()` with some files replaced (None deletes one)."""
        files = clean_files()
        for stem, text in changes.items():
            if text is None:
                files.pop(stem, None)
            else:
                files[stem] = text
        return self.check(files, *extra)

    def assertFails(self, result: tuple[int, str, str], *needles: str) -> None:
        code, out, err = result
        self.assertEqual(code, 1, f"expected exit 1\nstdout:\n{out}\nstderr:\n{err}")
        for needle in needles:
            self.assertIn(needle, err)

    def assertPasses(self, result: tuple[int, str, str]) -> None:
        code, out, err = result
        self.assertEqual(code, 0, f"expected exit 0\nstdout:\n{out}\nstderr:\n{err}")


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
        code, _out, err = self.check(
            {
                "dynamixel": '(models: { 1020: (family: "x-series", name: "A"), 1020: (family: "x-series", name: "B") })',
            }
        )
        self.assertEqual(code, 2)
        self.assertIn("dynamixel.ron", err)
        self.assertIn("1020", err)

    def test_cli_unterminated_string_exits_2(self) -> None:
        code, _out, err = self.check(
            {"dynamixel": "(models: {})", "broken": '(\n  source: "open\n)\n'}
        )
        self.assertEqual(code, 2)
        self.assertIn("broken.ron:2", err)


class CliTests(FixtureCase):
    """Report frame, exit codes and options (contract table-tools section 1)."""

    def test_clean_fixture_exits_0(self) -> None:
        self.assertPasses(self.check(clean_files()))

    def test_missing_map_exits_2(self) -> None:
        code, _out, err = self.check_with(dynamixel=None)
        self.assertEqual(code, 2)
        self.assertIn("dynamixel.ron", err)

    def test_unparseable_file_exits_2_naming_it(self) -> None:
        code, _out, err = self.check_with(x="( source: Unverified, control_table: [ { address: 0 ")
        self.assertEqual(code, 2)
        self.assertIn("x.ron", err)

    def test_bad_arguments_exit_2(self) -> None:
        for argv in (["--today", "28.10.2026"], ["--today", "2026-02-30"], ["--bogus"]):
            with self.subTest(argv=argv):
                code, _out, _err = run_check(["--dir", str(self.dir), *argv])
                self.assertEqual(code, 2)

    def test_sections_in_order(self) -> None:
        code, out, _err = self.check(clean_files())
        self.assertEqual(code, 0)
        heads = [line for line in out.splitlines() if line and not line.startswith(" ")]
        self.assertEqual(heads[:3], ["tables", "dual-protocol models", "errors"])
        self.assertTrue(heads[3].startswith("summary: 3 tables,"), heads)
        self.assertEqual(len(heads), 4)
        self.assertIn("0 errors", heads[3])

    def test_errors_go_to_stdout_and_stderr(self) -> None:
        result = self.check_with(x="( source: Unverified, protocols: [P1, P2] )")
        self.assertFails(result, "x.ron")
        _code, out, err = result
        section = out.split("errors\n", 1)[1].split("summary:", 1)[0]
        error_lines = [line.strip() for line in section.splitlines() if line.strip()]
        self.assertTrue(error_lines)
        self.assertEqual(error_lines, [line for line in err.splitlines() if line])
        self.assertTrue(all(line.startswith("x.ron") for line in error_lines))

    def test_stems_limit_only_the_table_section(self) -> None:
        files = clean_files()
        files["x"] = "( source: Unverified, protocols: [P1, P2] )"
        code, out, _err = self.check(files, "ax")
        self.assertEqual(code, 1)
        section = out.split("dual-protocol models", 1)[0]
        self.assertIn("  ax ", section)
        self.assertNotIn("xl320", section)
        self.assertIn("x.ron", out.split("errors\n", 1)[1])

    def test_unknown_stem_exits_2(self) -> None:
        code, _out, _err = self.check(clean_files(), "nope")
        self.assertEqual(code, 2)

    def test_markdown_prints_a_list(self) -> None:
        code, out, _err = self.check(clean_files(), "--markdown", "ax", "x")
        self.assertEqual(code, 0)
        lines = [line for line in out.splitlines() if line]
        self.assertEqual(len(lines), 2)
        self.assertTrue(all(line.startswith("- `") for line in lines), lines)
        self.assertTrue(lines[0].startswith("- `ax`"))


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

    def test_missing_source_fails_naming_the_file(self) -> None:  # AS2
        self.assertFails(self.check_with(ax=self.ax(source=None)), "ax.ron", "`source`")

    def test_unverified_is_listed_not_an_error(self) -> None:  # AS3
        code, out, _err = self.check_with()
        self.assertEqual(code, 0)
        self.assertRegex(out, r"\n  ax\s+unverified\s")

    def test_verified_lists_url_and_date(self) -> None:  # AS1
        result = self.check_with(ax=self.ax(source=verified(), instructions=ax_instructions()))
        self.assertPasses(result)
        self.assertIn(f"verified {AX_URL} {TODAY}", result[1])

    def test_url_on_another_host_fails(self) -> None:  # AS4
        for url in ("https://example.com/ax-12a/", "http://emanual.robotis.com/docs/en/dxl/ax/ax-12a/"):
            with self.subTest(url=url):
                self.assertFails(
                    self.check_with(ax=self.ax(source=verified(url=url), instructions=ax_instructions())),
                    "ax.ron",
                )

    def test_impossible_date_fails(self) -> None:
        self.assertFails(
            self.check_with(ax=self.ax(source=verified(date="2026-02-30"), instructions=ax_instructions())),
            "ax.ron",
        )

    def test_future_date_fails(self) -> None:
        self.assertFails(
            self.check_with(ax=self.ax(source=verified(date="2026-10-29"), instructions=ax_instructions())),
            "ax.ron",
            "future",
        )

    def test_non_iso_date_fails(self) -> None:
        self.assertFails(
            self.check_with(ax=self.ax(source=verified(date="28.10.2026"), instructions=ax_instructions())),
            "ax.ron",
        )

    def test_missing_verified_field_fails(self) -> None:
        self.assertFails(
            self.check_with(ax=self.ax(source=f'Verified(url: "{AX_URL}")', instructions=ax_instructions())),
            "ax.ron",
        )

    def test_other_tag_fails(self) -> None:
        self.assertFails(self.check_with(ax=self.ax(source="Checked")), "ax.ron")
        self.assertFails(self.check_with(ax=self.ax(source='"Unverified"')), "ax.ron")

    def test_markdown_shows_flag_url_and_date(self) -> None:
        code, out, _err = self.check_with(
            "--markdown", "ax", "x", ax=self.ax(source=verified(), instructions=ax_instructions())
        )
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
            self.check_with(x=self.x(self.both(), source="Unverified")), "x.ron", "verified tables only"
        )

    def test_verified_table_needs_an_entry_per_protocol(self) -> None:
        self.assertFails(self.check_with(x=self.x([instr("P1")])), "x.ron", "P2")
        self.assertFails(self.check_with(x=self.x(None)), "x.ron", "`instructions`")

    def test_protocol_must_be_in_protocols(self) -> None:
        ax = table(
            AX_ROWS,
            protocols="[P1]",
            type_info=AX_SCALE,
            source=verified(),
            instructions="[" + instr("P1") + ", " + instr("P2", source=P2_URL) + "]",
        )
        self.assertFails(self.check_with(ax=ax), "ax.ron", "P2")

    def test_source_must_be_on_the_emanual(self) -> None:
        self.assertFails(
            self.check_with(x=self.x([instr("P1", source="https://github.com/ROBOTIS-GIT/"), self.both()[1]])),
            "x.ron",
        )

    def test_list_non_empty_known_names_no_duplicates(self) -> None:
        p2 = self.both()[1]
        for names in ("", "Ping, Ping", "Ping, Teleport", '"Ping"'):
            with self.subTest(names=names):
                self.assertFails(self.check_with(x=self.x([instr("P1", names), p2])), "x.ron")

    def test_p1_entries_only_use_protocol_1_instructions(self) -> None:
        p2 = self.both()[1]
        self.assertFails(self.check_with(x=self.x([instr("P1", "Ping, SyncRead"), p2])), "x.ron", "SyncRead")
        self.assertPasses(
            self.check_with(
                x=self.x(
                    [instr("P1", "Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, SyncWrite, BulkRead"), p2]
                )
            )
        )

    def test_entries_of_one_protocol_differ_in_from_fw(self) -> None:
        p2 = self.both()[1]
        self.assertFails(self.check_with(x=self.x([instr("P1"), instr("P1", "Reboot"), p2])), "x.ron", "from_fw")
        self.assertPasses(self.check_with(x=self.x([instr("P1"), instr("P1", "Reboot", from_fw=40), p2])))
        self.assertFails(
            self.check_with(x=self.x([instr("P1"), instr("P1", "Reboot", from_fw=40), instr("P1", "BulkRead", from_fw=40), p2])),
            "x.ron",
        )

    def test_entry_must_be_a_record_with_known_fields(self) -> None:
        p2 = self.both()[1]
        self.assertFails(self.check_with(x=self.x(["P1", p2])), "x.ron")
        self.assertFails(
            self.check_with(x=self.x([f'(protocol: P1, list: [Ping])', p2])), "x.ron", "`source`"
        )
        self.assertFails(self.check_with(x=self.x([instr("P1", from_fw=300), p2])), "x.ron", "from_fw")


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
        for protocols in (None, "[]", "[P1, P1]", "[P3]", "P1", "[P1, P2, P1]"):
            with self.subTest(protocols=protocols):
                self.assertFails(self.check_with(x=table(protocols=protocols)), "x.ron", "`protocols`")

    def test_column_shows_protocols(self) -> None:
        code, out, _err = self.check_with()
        self.assertEqual(code, 0)
        self.assertRegex(out, r"\n  x\s+unverified\s+\[P1, P2\]\s")
        self.assertRegex(out, r"\n  xl320\s+unverified\s+\[P2\]\s")
        self.assertRegex(out, r"\n  ax\s+unverified\s+\[P1\]\s")


class RowTests(FixtureCase):
    """Row `access` and `area`: US3 AS2, FR-007, data-model "Register row"."""

    def test_missing_access_or_area_names_the_row(self) -> None:
        rows = replaced(X_ROWS, "Goal Position", access=None)
        self.assertFails(self.check_with(x=table(rows)), 'x.ron: row 116 "Goal Position": missing `access`')
        rows = replaced(X_ROWS, "Goal Position", area=None)
        self.assertFails(self.check_with(x=table(rows)), 'x.ron: row 116 "Goal Position": missing `area`')

    def test_other_access_or_area_values_fail(self) -> None:
        self.assertFails(self.check_with(x=table(replaced(X_ROWS, "Present Position", access="W"))), "x.ron", "W")
        self.assertFails(self.check_with(x=table(replaced(X_ROWS, "Present Position", area="ROM"))), "x.ron", "ROM")

    def test_hybrid_area_is_allowed(self) -> None:
        rows = X_ROWS + [(212, 4, "Velocity I Gain", "RW", "Hybrid")]
        self.assertPasses(self.check_with(x=table(rows)))

    def test_hybrid_is_not_a_valid_area_for_a_role(self) -> None:
        rows = replaced(X_ROWS, "Goal Position", area="Hybrid")
        self.assertFails(self.check_with(x=table(rows)), "x.ron", "Goal Position")

    def test_rows_are_checked_in_unmapped_tables_too(self) -> None:
        rows = replaced(X_ROWS, "Goal Position", area=None)
        result = self.check_with(
            extra=table(rows),
            dynamixel=model_map(
                {12: entry("ax", ("p1",), family="ax"), 350: entry("xl320", ("p2",)), 1020: entry("x")},
                ['(file: "extra", reason: NoModelNumber)'],
            ),
        )
        self.assertFails(result, "extra.ron", "missing `area`")


class RoleTests(FixtureCase):
    """Role rules on mapped tables: contract 4.3, xtask checks 10–12, 15, 16."""

    def test_missing_role_fails(self) -> None:
        for name in ("Torque Enable", "Goal Position", "Present Position", "Firmware Version", "Model Number"):
            with self.subTest(name=name):
                self.assertFails(self.check_with(x=table(without(X_ROWS, name))), "x.ron", name)

    def test_exactly_one_complete_limit_form(self) -> None:
        both = AX_ROWS[:2] + X_ROWS[3:5] + AX_ROWS[2:]
        self.assertFails(
            self.check_with(ax=table(both, protocols="[P1]", type_info=AX_SCALE)), "ax.ron", "limit"
        )
        half = without(AX_ROWS, "CCW Angle Limit")
        self.assertFails(self.check_with(ax=table(half, protocols="[P1]", type_info=AX_SCALE)), "ax.ron", "limit")
        half_minmax = without(X_ROWS, "Min Position Limit")
        self.assertFails(self.check_with(x=table(half_minmax)), "x.ron", "limit")

    def test_role_attributes(self) -> None:
        cases = [
            ("Torque Enable", {"access": "R"}),
            ("Torque Enable", {"area": "EEPROM"}),
            ("Goal Position", {"access": "R"}),
            ("Goal Position", {"area": "EEPROM"}),
            ("Present Position", {"access": "RW"}),
            ("Present Position", {"area": "EEPROM"}),
            ("Max Position Limit", {"access": "R"}),
            ("Min Position Limit", {"area": "RAM"}),
        ]
        for name, change in cases:
            with self.subTest(name=name, change=change):
                self.assertFails(self.check_with(x=table(replaced(X_ROWS, name, **change))), "x.ron", name)
        ax = table(replaced(AX_ROWS, "CW Angle Limit", access="R"), protocols="[P1]", type_info=AX_SCALE)
        self.assertFails(self.check_with(ax=ax), "ax.ron", "CW Angle Limit")

    def test_model_number_at_0_size_2_r_eeprom(self) -> None:
        for change in ({"access": "RW"}, {"area": "RAM"}, {"address": 4}, {"size": 4}):
            with self.subTest(change=change):
                self.assertFails(
                    self.check_with(x=table(replaced(X_ROWS, "Model Number", **change))), "x.ron", "Model Number"
                )

    def test_scale_sanity(self) -> None:
        for scale in (
            (2048, 4095, 0, -3.1415927, 3.1400587),  # max_raw < min_raw
            (2048, 0, 4095, 3.1400587, -3.1415927),  # max_radian < min_radian
            (5000, 0, 4095, -3.1415927, 3.1400587),  # zero above max
            (-1, 0, 4095, -3.1415927, 3.1400587),  # zero below min
        ):
            with self.subTest(scale=scale):
                self.assertFails(self.check_with(x=table(type_info=scale)), "x.ron", "type_info")
        self.assertFails(self.check_with(x=table(type_info=None)), "x.ron", "type_info")

    def test_duplicate_register_constant(self) -> None:
        rows = X_ROWS + [(200, 4, "Goal-Position", "RW", "RAM")]
        self.assertFails(self.check_with(x=table(rows)), "x.ron", "GOAL_POSITION")
        rows = X_ROWS + [(116, 4, "Goal Position Alias", "RW", "RAM")]
        self.assertPasses(self.check_with(x=table(rows)))

    def test_unmapped_table_is_not_role_checked(self) -> None:
        rows = without(X_ROWS, "Goal Position", "Torque Enable") + [(200, 4, "Goal-Position", "RW", "RAM")]
        rows = rows + [(204, 4, "Goal Position", "RW", "RAM")]
        result = self.check_with(
            extra=table(rows, type_info=None),
            dynamixel=model_map(
                {12: entry("ax", ("p1",), family="ax"), 350: entry("xl320", ("p2",)), 1020: entry("x")},
                ['(file: "extra", reason: NoModelNumber)'],
            ),
        )
        self.assertPasses(result)


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
        code, out, _err = self.check_with()
        self.assertEqual(code, 0, out)

    def test_all_three_lists_empty_passes(self) -> None:
        self.assertPasses(self.check_with(x=table(operating_modes="(bounded: [], unbounded: [], no_position: [])")))

    # rows
    def test_min_max_table_without_operating_mode_has_no_mode_source(self) -> None:
        rows = without(X_ROWS, "Operating Mode")
        self.assertFails(self.check_with(x=table(rows, operating_modes=None)), "x.ron", "no mode source")

    def test_both_mode_names_fail(self) -> None:
        rows = X_ROWS + [(12, 1, "Control Mode", "RW", "EEPROM")]
        self.assertFails(self.check_with(x=table(rows, operating_modes=X_MODES)), "x.ron", "Control Mode")

    def test_operating_mode_beside_cw_ccw_pair_fails(self) -> None:
        rows = AX_ROWS + [(11, 1, "Operating Mode", "RW", "EEPROM")]
        self.assertFails(self.check_with(ax=self.ax(rows, operating_modes=X_MODES)), "ax.ron", "CW/CCW")

    def test_two_operating_mode_rows_in_unmapped_table_fail(self) -> None:
        rows = X_ROWS + [(12, 1, "Operating Mode", "RW", "EEPROM")]
        self.assertFails(self.check(unmapped(extra=table(rows))), "extra.ron", "Operating Mode")

    def test_control_mode_only_in_xl320(self) -> None:
        files = unmapped(xl330_x=table(XL320_ROWS, protocols="[P2]", type_info=AX_SCALE))
        self.assertFails(self.check(files), "xl330_x.ron", "Control Mode")

    def test_mode_row_attributes(self) -> None:
        for change in ({"size": 2}, {"access": "R"}, {"area": "RAM"}):
            with self.subTest(change=change):
                rows = replaced(X_ROWS, "Operating Mode", **change)
                self.assertFails(self.check_with(x=table(rows)), "x.ron", "Operating Mode")
                self.assertFails(self.check(unmapped(extra=table(rows))), "extra.ron", "Operating Mode")

    def test_p1_mode_row_above_255_fails(self) -> None:
        rows = replaced(X_ROWS, "Operating Mode", address=300)
        self.assertFails(self.check_with(x=table(rows)), "x.ron", "255")

    # operating_modes
    def test_mode_row_without_operating_modes_fails(self) -> None:
        self.assertFails(self.check_with(x=table(operating_modes=None)), "x.ron", "`operating_modes`")
        self.assertFails(self.check(unmapped(extra=table(operating_modes=None))), "extra.ron", "`operating_modes`")

    def test_operating_modes_without_mode_row_fails(self) -> None:
        bad = self.ax(operating_modes="(bounded: [3], unbounded: [], no_position: [])")
        self.assertFails(self.check_with(ax=bad), "ax.ron", "`operating_modes`")

    def test_value_in_two_lists_names_value_and_lists(self) -> None:
        modes = "(bounded: [3, 4], unbounded: [4, 5], no_position: [0, 1, 16])"
        self.assertFails(
            self.check_with(x=table(operating_modes=modes)), "x.ron", "value 4", "`bounded`", "`unbounded`"
        )

    def test_malformed_operating_modes_fail(self) -> None:
        for modes in (
            "(bounded: [3], unbounded: [4, 5])",
            "(bounded: [3], unbounded: [4, 5], no_position: [0, 256])",
            "(bounded: [3], unbounded: [4, 5], no_position: [-1])",
            "(bounded: 3, unbounded: [4, 5], no_position: [0])",
            "(bounded: [3], unbounded: [4, 5], no_position: [0], extra: [])",
            "[3, 4, 5]",
        ):
            with self.subTest(modes=modes):
                self.assertFails(self.check_with(x=table(operating_modes=modes)), "x.ron", "`operating_modes`")


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

    def test_from_fw_order(self) -> None:
        ok = f'(family: "x-series", name: "X", p2: {ref_list((0, "x_fw52"), (53, "x"))}, tables: {ref_list((0, "x_fw52"), (53, "x"))})'
        files = {"x": table(protocols="[P2]"), "x_fw52": table(protocols="[P2]")}
        self.assertPasses(self.run_map({1190: ok}, files))
        first_one = f'(family: "x-series", name: "X", p2: {ref_list((1, "x"))}, tables: {ref_list((1, "x"))})'
        self.assertFails(self.run_map({1190: first_one}, {"x": table(protocols="[P2]")}), "model 1190", "from_fw")
        swapped = ref_list((0, "x"), (53, "x_fw52"), (40, "x2"))
        files = {"x": table(protocols="[P2]"), "x_fw52": table(protocols="[P2]"), "x2": table(protocols="[P2]")}
        self.assertFails(
            self.run_map({1190: f'(family: "x-series", name: "X", p2: {swapped}, tables: {swapped})'}, files),
            "model 1190",
            "from_fw",
        )

    def test_missing_file(self) -> None:
        self.assertFails(self.run_map({1020: entry("x"), 1030: entry("gone")}), "model 1030", "gone")

    def test_needs_a_non_empty_protocol_list(self) -> None:
        self.assertFails(self.run_map({1020: entry("x", (), tables=ref_list("x"))}), "model 1020", "p1")
        empty = '(family: "x-series", name: "X", p1: [], p2: [(from_fw: 0, file: "x")], default: P2, tables: [(from_fw: 0, file: "x")])'
        self.assertFails(self.run_map({1020: empty}), "model 1020", "empty")

    def test_default_exactly_with_both_lists_and_p2(self) -> None:
        self.assertFails(self.run_map({1020: entry("x", default=None)}), "model 1020", "default")
        self.assertFails(self.run_map({1020: entry("x", default="P1")}), "model 1020", "default")
        one = {"x": table(protocols="[P2]")}
        self.assertFails(self.run_map({1020: entry("x", ("p2",), default="P2")}, one), "model 1020", "default")

    def test_family(self) -> None:
        self.assertFails(self.run_map({1020: entry("x", family="dxl")}), "model 1020", "family")
        for family in ("ax", "mx", "rx", "x-series", "y-series", "pro"):
            with self.subTest(family=family):
                self.assertPasses(self.run_map({1020: entry("x", family=family)}))

    def test_file_under_one_number_only(self) -> None:
        self.assertFails(self.run_map({1020: entry("x"), 1030: entry("x")}), "x", "1020", "1030")
        self.assertPasses(self.run_map({1020: entry("x")}))

    def test_list_needs_the_protocol_in_the_table(self) -> None:
        self.assertFails(self.run_map({1020: entry("x", ("p1",))}, {"x": table(protocols="[P2]")}), "model 1020", "P1")
        self.assertFails(self.run_map({1020: entry("x", ("p2",))}, {"x": table(protocols="[P1]")}), "model 1020", "P2")

    def test_dual_protocol_table_needs_both_lists(self) -> None:
        self.assertFails(self.run_map({1020: entry("x", ("p2",))}), "model 1020", "both")

    def test_legacy_tables_equals_the_default_list(self) -> None:
        files = {"x": table(), "y": table()}
        other = ref_list("y")
        self.assertFails(self.run_map({1020: entry("x", tables=other), 1030: entry("y")}, files), "model 1020", "tables")
        self.assertFails(self.run_map({1020: entry("x", tables=None)}), "model 1020", "tables")

    def test_firmware_version_at_one_place_per_list(self) -> None:
        moved = replaced(X_ROWS, "Firmware Version", address=7)
        files = {"x": table(protocols="[P2]"), "x_fw52": table(moved, protocols="[P2]")}
        refs = ref_list((0, "x_fw52"), (53, "x"))
        self.assertFails(
            self.run_map({1190: f'(family: "x-series", name: "X", p2: {refs}, tables: {refs})'}, files),
            "model 1190",
            "Firmware Version",
        )

    def test_p1_role_above_255(self) -> None:
        rows = replaced(X_ROWS, "Goal Position", address=300)
        self.assertFails(self.run_map({1020: entry("x")}, {"x": table(rows)}), "x.ron", "255")
        self.assertPasses(self.run_map({1020: entry("x", ("p2",))}, {"x": table(rows, protocols="[P2]")}))

    def test_type_and_module_identifiers(self) -> None:
        files = {"ax_12a": table(), "ax12a": table()}
        self.assertFails(self.run_map({1: entry("ax_12a"), 2: entry("ax12a")}, files), "Ax12a")
        self.assertFails(self.run_map({1: entry("tests")}, {"tests": table()}), "tests")
        self.assertFails(self.run_map({1: entry("type")}, {"type": table()}), "type")
        self.assertPasses(self.run_map({1090: entry("2xl430_w250")}, {"2xl430_w250": table()}))

    def test_dual_protocol_section(self) -> None:
        code, out, _err = self.check(clean_files())
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
        self.assertFails(self.run_nm([], extra=table()), "extra.ron", "not_mapped")
        self.assertFails(self.run_nm(None, extra=table()), "extra.ron", "not_mapped")

    def test_recorded_table_passes_and_shows_its_reason(self) -> None:
        result = self.run_nm(['(file: "extra", reason: SameNumberAs(12))', '(file: "other", reason: NoModelNumber)'],
                             extra=table(), other=table())
        self.assertPasses(result)
        self.assertRegex(result[1], r"\n  extra\s+unverified\s+\[P1, P2\]\s+not mapped: same number as 12\n")
        self.assertRegex(result[1], r"\n  other\s+unverified\s+\[P1, P2\]\s+not mapped: no model number\n")
        self.assertIn("2 not mapped", result[1])

    def test_mapped_and_recorded_fails(self) -> None:
        self.assertFails(self.run_nm(['(file: "x", reason: SameNumberAs(12))']), "x", "not_mapped")

    def test_record_for_missing_file_fails(self) -> None:
        self.assertFails(self.run_nm(['(file: "gone", reason: NoModelNumber)']), "gone")

    def test_same_number_as_unknown_number_fails(self) -> None:
        self.assertFails(self.run_nm(['(file: "extra", reason: SameNumberAs(99))'], extra=table()), "99")

    def test_other_reasons_fail(self) -> None:
        for reason in ("Unverified", "SameNumberAs(70000)", "SameNumberAs()", '"NoModelNumber"', "SameNumberAs(12, 13)"):
            with self.subTest(reason=reason):
                self.assertFails(self.run_nm([f'(file: "extra", reason: {reason})'], extra=table()), "extra")

    def test_record_listed_twice_fails(self) -> None:
        records = ['(file: "extra", reason: NoModelNumber)', '(file: "extra", reason: NoModelNumber)']
        self.assertFails(self.run_nm(records, extra=table()), "extra")


if __name__ == "__main__":
    unittest.main()

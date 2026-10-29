"""Tests for `emanual.py` (specs/005-tables/contracts/table-tools.md section 2).

The pages are small hand-written HTML snippets shaped like the AX, X and PRO+
pages; the fetch function is patched, so no test needs the network.
"""

from __future__ import annotations

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))

import emanual  # noqa: E402
from check_tables import Ident, parse_ron  # noqa: E402

AX_URL = "https://emanual.robotis.com/docs/en/dxl/ax/ax-test/"
X_URL = "https://emanual.robotis.com/docs/en/dxl/x/x-test/"
PRO_URL = "https://emanual.robotis.com/docs/en/dxl/p/p-test/"


def tr(*cells: str) -> str:
    return "<tr>" + "".join(f'<td style="text-align: left">{c}</td>' for c in cells) + "</tr>\n"


def th(*cells: str) -> str:
    return "<thead><tr>" + "".join(f"<th>{c}</th>" for c in cells) + "</tr></thead>\n"


AX_HEAD = th("Address", "Size(Byte)", "Data Name", "Description", "Access", "Initial<br />Value", "Min", "Max")
X_HEAD = th("Address", "Size(Byte)", "Data Name", "Access", "Initial<br />Value", "Range", "Unit")
PRO_HEAD = th(
    "Address", "Size(Byte)", "Modbus<br />Address", "Data Name", "Access", "Initial<br />Value", "Range", "Unit"
)


def ax_page(ram_access: str = "RW", heading: str = "Control Table of RAM Area") -> str:
    return (
        "<html><body><h1>AX-TEST</h1>\n"
        '<h2 id="eeprom"><a href="#eeprom">Control Table of EEPROM Area</a></h2>\n<table>'
        + AX_HEAD
        + "<tbody>"
        + tr("0", "2", '<a href="#model-number">Model Number</a>', "Model Number", "R", "12", "-", "-")
        + tr("2", "1", '<a href="#fw">Firmware Version</a>', "Firmware Version", "R", "-", "-", "-")
        + tr("6", "2", "CW Angle Limit", "Clockwise Angle Limit", "RW", "0", "0", "1023")
        + "</tbody></table>\n"
        + f"<h2>{heading}</h2>\n<table>"
        + AX_HEAD
        + "<tbody>"
        + tr("24", "1", "Torque Enable", "Motor Torque On/Off", ram_access, "0", "0", "1")
        + tr("30", "2", "Goal Position", "Target Position", "RW", "-", "0", "1023")
        + tr("36", "2", "Present Position", "Present Position", "R", "-", "-", "-")
        + "</tbody></table>\n"
        + "<h2>Control Table Description</h2>\n"
        + "<table><thead><tr><th>Value</th><th>Baud Rate</th></tr></thead><tbody>"
        + "<tr><td>1</td><td>1M</td></tr></tbody></table>\n</body></html>"
    )


X_PAGE = (
    "<h2>Control Table of EEPROM Area</h2>\n<table>"
    + X_HEAD
    + "<tbody>"
    + tr("0", "2", "Model Number", "R", "1,020", "-", "-")
    + tr("6", "1", "Firmware Version", "R", "-", "-", "-")
    + tr("11", "1", "Operating Mode", "RW", "3", "0 ~ 16", "-")
    + "</tbody></table>\n<h2>Control Table of RAM Area</h2>\n<table>"
    + X_HEAD
    + "<tbody>"
    + tr("64", "1", "Torque Enable", "RW", "0", "0 ~ 1", "-")
    + tr("168", "2", "Indirect Address 1", "RW", "224", "64 ~ 661", "-")
    + tr("170", "2", "Indirect Address 2", "RW", "225", "64 ~ 661", "-")
    + tr("…", "…", "…", "…", "…", "-", "-")
    + tr("224", "1", "Indirect Data 1", "RW", "0", "0 ~ 255", "-")
    + "</tbody></table>\n"
)

PRO_PAGE = (
    "<h2>Control Table of EEPROM Area</h2>\n<table>"
    + PRO_HEAD
    + "<tbody>"
    + tr("0", "2", "40001", "Model Number", "R", "2,020", "-", "-")
    + tr("11", "1", "40012", "Operating Mode", "RW", "3", "0 ~ 16", "-")
    + "</tbody></table>\n<h2>Control Table of RAM Area</h2>\n<table>"
    + PRO_HEAD
    + "<tbody>"
    + tr("512", "1", "40257", "Torque Enable", "RW", "0", "0 ~ 1", "-")
    + "</tbody></table>\n"
)

ONE_LINE = """(
  source: Unverified,
  type_info: {
    value_of_zero_radian_position: 512,
    value_of_max_radian_position: 1023,
    value_of_min_radian_position: 0,
    min_radian: -2.6205530,
    max_radian: 2.6154347,
  },
  control_table: [
    // EEPROM area
    { address: 0, size: 2, data_name: "Model Number" },
    { address: 2, size: 1, data_name: "Firmware Version", access: R, area: EEPROM },
    { address: 6, size: 2, data_name: "CW Angle Limit" },
    // RAM area
    { address: 24, size: 1, data_name: "Torque Enable" },
    { address: 30, size: 2, data_name: "Goal Position" },
    { address: 36, size: 2, data_name: "Present Position" },
  ],
)
"""

MULTI_LINE = """(
  source: Unverified,
  control_table: [
    {
      address: 0,
      size: 2,
      data_name: "Model Number",
    },
    {
      address: 6,
      size: 1,
      data_name: "Firmware Version",
    },
    {
      address: 11,
      size: 1,
      data_name: "Operating Mode",
    },
    {
      address: 64,
      size: 1,
      data_name: "Torque Enable",
    },
    {
      address: 168,
      size: 2,
      data_name: "Indirect Address 1",
    },
    {
      address: 168,
      size: 2,
      data_name: "Indirect Address Write",
    },
    {
      address: 224,
      size: 1,
      data_name: "Indirect Data 1",
    },
  ],
)
"""


def run(argv: list[str], pages: dict[str, str], tables_dir: Path | None = None) -> tuple[int, str, str]:
    """Calls `emanual.main(argv)` with `fetch` served from `pages` (URL -> HTML)."""

    def fake_fetch(url: str) -> str:
        if url not in pages:
            raise emanual.FetchError(f"{url}: HTTP Error 404: Not Found")
        return pages[url]

    out, err = io.StringIO(), io.StringIO()
    patches = [mock.patch.object(emanual, "fetch", fake_fetch)]
    if tables_dir is not None:
        patches.append(mock.patch.object(emanual, "TABLES_DIR", tables_dir))
    with contextlib.ExitStack() as stack:
        for patch in patches:
            stack.enter_context(patch)
        stack.enter_context(contextlib.redirect_stdout(out))
        stack.enter_context(contextlib.redirect_stderr(err))
        code = emanual.main(argv)
    return code, out.getvalue(), err.getvalue()


class TableDirCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self._tmp.name)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def write(self, stem: str, text: str) -> Path:
        path = self.dir / f"{stem}.ron"
        path.write_text(text, encoding="utf-8")
        return path


class ParsePageTests(unittest.TestCase):
    def test_ax_shaped_page(self) -> None:
        self.assertEqual(
            emanual.parse_page(ax_page()),
            [
                (0, 2, "Model Number", "R", "EEPROM"),
                (2, 1, "Firmware Version", "R", "EEPROM"),
                (6, 2, "CW Angle Limit", "RW", "EEPROM"),
                (24, 1, "Torque Enable", "RW", "RAM"),
                (30, 2, "Goal Position", "RW", "RAM"),
                (36, 2, "Present Position", "R", "RAM"),
            ],
        )

    def test_x_shaped_page_skips_ellipsis_rows(self) -> None:
        self.assertEqual(
            emanual.parse_page(X_PAGE),
            [
                (0, 2, "Model Number", "R", "EEPROM"),
                (6, 1, "Firmware Version", "R", "EEPROM"),
                (11, 1, "Operating Mode", "RW", "EEPROM"),
                (64, 1, "Torque Enable", "RW", "RAM"),
                (168, 2, "Indirect Address 1", "RW", "RAM"),
                (170, 2, "Indirect Address 2", "RW", "RAM"),
                (224, 1, "Indirect Data 1", "RW", "RAM"),
            ],
        )

    def test_pro_plus_page_matches_columns_by_header_name(self) -> None:
        self.assertEqual(
            emanual.parse_page(PRO_PAGE),
            [
                (0, 2, "Model Number", "R", "EEPROM"),
                (11, 1, "Operating Mode", "RW", "EEPROM"),
                (512, 1, "Torque Enable", "RW", "RAM"),
            ],
        )

    def test_access_other_than_r_or_rw_is_an_error(self) -> None:
        with self.assertRaises(emanual.PageError) as ctx:
            emanual.parse_page(ax_page(ram_access="W"))
        self.assertIn("W", str(ctx.exception))

    def test_row_outside_an_eeprom_or_ram_heading_is_an_error(self) -> None:
        with self.assertRaises(emanual.PageError):
            emanual.parse_page(ax_page(heading="Control Table Description"))


class CommandTests(TableDirCase):
    def test_rows_prints_ron_rows_in_page_order(self) -> None:
        code, out, _err = run(["rows", AX_URL], {AX_URL: ax_page()})
        self.assertEqual(code, 0)
        lines = [line.strip() for line in out.splitlines() if line.strip().startswith("{")]
        self.assertEqual(lines[0], '{ address: 0, size: 2, data_name: "Model Number", access: R, area: EEPROM },')
        self.assertEqual(lines[3], '{ address: 24, size: 1, data_name: "Torque Enable", access: RW, area: RAM },')
        self.assertEqual(len(lines), 6)

    def test_rows_refuses_other_hosts(self) -> None:
        code, _out, err = run(["rows", "https://example.com/"], {})
        self.assertEqual(code, 2)
        self.assertIn("emanual.robotis.com", err)

    def test_rows_page_not_found_exits_2(self) -> None:
        code, _out, _err = run(["rows", AX_URL], {})
        self.assertEqual(code, 2)

    def test_rows_bad_access_exits_1(self) -> None:
        code, _out, _err = run(["rows", AX_URL], {AX_URL: ax_page(ram_access="W")})
        self.assertEqual(code, 1)

    def test_diff_reports_the_four_categories(self) -> None:
        self.write(
            "t",
            """(
  control_table: [
    { address: 0, size: 2, data_name: "Model Number", access: RW, area: EEPROM },
    { address: 2, size: 1, data_name: "Firmware Version" },
    { address: 6, size: 2, data_name: "CW Angle Limit" },
    { address: 24, size: 1, data_name: "Torque Enable" },
    { address: 30, size: 2, data_name: "Goal-Position" },
    { address: 50, size: 1, data_name: "Not On Page" },
  ],
)
""",
        )
        code, out, _err = run(["diff", "t", AX_URL], {AX_URL: ax_page()}, self.dir)
        self.assertEqual(code, 1)
        self.assertIn("only in table", out)
        self.assertIn('50 1 "Not On Page"', out)
        self.assertIn("only on page", out)
        self.assertIn('36 2 "Present Position"', out)
        self.assertIn("different name", out)
        self.assertIn('"Goal-Position"', out)
        self.assertIn("different access/area", out)
        self.assertIn('"Model Number"', out)

    def test_diff_clean_exits_0(self) -> None:
        self.write("t", ONE_LINE)
        code, out, _err = run(["diff", "t", AX_URL], {AX_URL: ax_page()}, self.dir)
        self.assertEqual(code, 0, out)

    def test_diff_uses_the_page_list(self) -> None:
        self.write("t", ONE_LINE)
        with mock.patch.dict(emanual.PAGES, {"t": AX_URL}):
            code, _out, _err = run(["diff", "t"], {AX_URL: ax_page()}, self.dir)
        self.assertEqual(code, 0)

    def test_diff_without_url_or_page_entry_exits_2(self) -> None:
        self.write("t", ONE_LINE)
        code, _out, _err = run(["diff", "t"], {}, self.dir)
        self.assertEqual(code, 2)

    def test_fill_one_line_table(self) -> None:
        path = self.write("t", ONE_LINE)
        code, out, _err = run(["fill", "t", AX_URL], {AX_URL: ax_page()}, self.dir)
        self.assertEqual(code, 0, out)
        text = path.read_text(encoding="utf-8")
        self.assertIn('    { address: 0, size: 2, data_name: "Model Number", access: R, area: EEPROM },\n', text)
        self.assertIn('    { address: 24, size: 1, data_name: "Torque Enable", access: RW, area: RAM },\n', text)
        self.assertIn('    { address: 6, size: 2, data_name: "CW Angle Limit", access: RW, area: EEPROM },\n', text)
        self.assertEqual(text.count("access:"), 6)
        self.assertIn("    // RAM area\n", text)
        self.assertIn("min_radian: -2.6205530,", text)
        table = parse_ron(text, "t.ron")
        for row in table["control_table"]:
            self.assertIn(row["access"], (Ident("R"), Ident("RW")))
            self.assertIn(row["area"], (Ident("EEPROM"), Ident("RAM")))
        self.assertEqual(
            text.replace(", access: R, area: EEPROM", "")
            .replace(", access: RW, area: EEPROM", "")
            .replace(", access: RW, area: RAM", "")
            .replace(", access: R, area: RAM", ""),
            ONE_LINE.replace(", access: R, area: EEPROM", ""),
        )

    def test_fill_multi_line_table_keeps_style_and_lists_left_rows(self) -> None:
        path = self.write("t", MULTI_LINE)
        code, out, _err = run(["fill", "t", X_URL], {X_URL: X_PAGE}, self.dir)
        self.assertEqual(code, 0, out)
        text = path.read_text(encoding="utf-8")
        self.assertIn(
            """    {
      address: 11,
      size: 1,
      data_name: "Operating Mode",
      access: RW,
      area: EEPROM,
    },
""",
            text,
        )
        self.assertIn(
            """    {
      address: 168,
      size: 2,
      data_name: "Indirect Address Write",
    },
""",
            text,
        )
        self.assertIn("Indirect Address Write", out)
        self.assertIn("left", out)
        table = parse_ron(text, "t.ron")
        rows = {row["data_name"]: row for row in table["control_table"]}
        self.assertEqual(rows["Indirect Address 1"]["area"], Ident("RAM"))
        self.assertNotIn("access", rows["Indirect Address Write"])
        self.assertEqual(list(rows["Torque Enable"]), ["address", "size", "data_name", "access", "area"])

    def test_fill_stops_on_bad_access_and_writes_nothing(self) -> None:
        path = self.write("t", ONE_LINE)
        code, _out, _err = run(["fill", "t", AX_URL], {AX_URL: ax_page(ram_access="W")}, self.dir)
        self.assertEqual(code, 1)
        self.assertEqual(path.read_text(encoding="utf-8"), ONE_LINE)

    def test_fill_stops_on_row_without_area_and_writes_nothing(self) -> None:
        path = self.write("t", ONE_LINE)
        page = ax_page(heading="Control Table Description")
        code, _out, _err = run(["fill", "t", AX_URL], {AX_URL: page}, self.dir)
        self.assertEqual(code, 1)
        self.assertEqual(path.read_text(encoding="utf-8"), ONE_LINE)

    def test_fill_pro_plus_page(self) -> None:
        path = self.write(
            "t",
            """(
  control_table: [
    {
      address: 512,
      size: 1,
      data_name: "Torque Enable",
    },
  ],
)
""",
        )
        code, _out, _err = run(["fill", "t", PRO_URL], {PRO_URL: PRO_PAGE}, self.dir)
        self.assertEqual(code, 0)
        self.assertIn("      access: RW,\n      area: RAM,\n", path.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()

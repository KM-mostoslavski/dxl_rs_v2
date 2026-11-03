// C3/C4 for tables: written by /speckit-ov-c4-code
!element dxl.tables {

    // ---------------------------------------------------------------- C3
    tables_files = component "tables: Control table files" "76 per-model register tables (5 verified: ax_12a, ax_12_plus, mx_64_p1, mx_64, rx_28; 71 flagged Unverified). Each holds source, protocols, optional instructions (verified only), operating_modes (absent on AX, RX, MX-P1), type_info position scale, unit_info and control_table rows (address, size, data_name, access R|RW, area EEPROM|RAM|Hybrid)." "RON: control_tables/<model>.ron"
    tables_map = component "tables: Model map" "Model number to entry (family, name, also, per-protocol p1/p2 table lists ordered by from_fw, default P2 when both) plus not_mapped with a reason for every table no number points to. 73 tables mapped, 3 not_mapped." "RON: control_tables/dynamixel.ron"
    tables_checker = component "tables: Table checker" "Offline validator for every table and the map: RON subset parser, schema, source and date, protocols, instructions, row fields, operating-mode source, role rows, type_info, map and not_mapped rules, Rust identifier clashes. Exit 0 ok, 1 rule broken, 2 unreadable input." "Python 3 stdlib: control_tables/check_tables.py"
    tables_emanual_reader = component "tables: e-Manual reader" "Maintainer tool (network): extracts control-table rows from a model's e-Manual page, diffs them against a .ron table, and fills missing access/area fields without touching address, size or name." "Python 3 stdlib (urllib, html.parser): control_tables/emanual.py"
    tables_analyzer = component "tables: Layout analyzer" "Groups all tables into exact-duplicate layouts, per-register address/size consistency, aliases and protocol split; writes the analysis reports (currently 76 tables, 18 layouts, 142 register names)." "Python 3: control_tables/analyze_tables.py"
    tables_tool_tests = component "tables: Tool tests" "Offline unittest suites for the checker (93 tests) and the e-Manual reader (22 tests, no network)." "Python unittest: control_tables/test_*.py"

    tables_map -> tables_files "Maps model numbers and firmware ranges to" "file stems"
    tables_checker -> tables_files "Parses and validates"
    tables_checker -> tables_map "Parses and cross-checks against the tables"
    tables_emanual_reader -> tables_files "Diffs rows against and fills access/area into"
    tables_emanual_reader -> tables_checker "Reuses the RON parser of" "Python import"
    tables_analyzer -> tables_files "Reads register rows and protocols from"
    tables_analyzer -> tables_map "Reads model numbers from"
    tables_analyzer -> tables_checker "Reuses the RON parser of" "Python import"
    tables_tool_tests -> tables_checker "Tests" "unittest"
    tables_tool_tests -> tables_emanual_reader "Tests offline" "unittest"

    // ---------------------------------------------------------------- C4
    group "tables: Control table files" {
        tables_ron_schema = component "control_tables/<model>.ron" "Table schema pattern: source Unverified | Verified(url, verified), protocols [P1|P2], instructions (verified only), operating_modes (bounded, unbounded, no_position), type_info, unit_info, control_table rows. xtask ignores unit_info and instructions." "RON record" {
            tags "Code"
        }
        tables_ax_12a = component "control_tables/ax_12a.ron" "AX-12A, verified 2026-10-28, P1 only; no Operating Mode, mode source is the CW/CCW Angle Limit pair." "RON table" {
            tags "Code"
        }
        tables_ax_12_plus = component "control_tables/ax_12_plus.ron" "AX-12+, verified; same layout and model number as AX-12A, so listed in not_mapped as SameNumberAs(12)." "RON table" {
            tags "Code"
        }
        tables_mx_64_p1 = component "control_tables/mx_64_p1.ron" "MX-64 (model 310), verified, Protocol 1 firmware table." "RON table" {
            tags "Code"
        }
        tables_mx_64 = component "control_tables/mx_64.ron" "MX-64(2.0) (model 311), verified, P1 and P2, default P2; has Operating Mode." "RON table" {
            tags "Code"
        }
        tables_xm430_w350 = component "control_tables/xm430_w350.ron" "XM430-W350 (model 1020), unverified, P1 and P2; representative X-series 430 layout." "RON table" {
            tags "Code"
        }
        tables_xl330_fw_variants = component "control_tables/xl330_m077_fw52.ron + xl330_m077.ron" "Table variant pair: one model number (1190) with a table up to fw 52 and another from fw 53 (from_fw lists in the map)." "RON tables" {
            tags "Code"
        }
    }

    group "tables: Model map" {
        tables_map_models = component "control_tables/dynamixel.ron: models" "u16 model number -> (family ax|mx|rx|x-series|y-series|pro, name, also, p1, p2, default)." "RON map" {
            tags "Code"
        }
        tables_map_not_mapped = component "control_tables/dynamixel.ron: not_mapped" "Tables left out with a reason: NoModelNumber or SameNumberAs(n); today ax_12_plus, ax_18f, xw430_t200." "RON list" {
            tags "Code"
        }
    }

    group "tables: Table checker" {
        tables_check_parse_ron = component "check_tables.py::parse_ron" "RON subset tokenizer and parser into Record, Ident, Tagged; RonError names file, line and column." "Python function" {
            tags "Code"
        }
        tables_check_load = component "check_tables.py::load" "Loads dynamixel.ron and every table in the directory; LoadError when the map is missing." "Python function" {
            tags "Code"
        }
        tables_check_run_checks = component "check_tables.py::run_checks" "Runs per-table rules (shape, source, protocols, instructions, row fields, mode source, roles, type_info) then map and not_mapped rules into a Report." "Python function" {
            tags "Code"
        }
        tables_check_map_info = component "check_tables.py::MapInfo" "Map summary from read_map: which stems are mapped, under p1/p2, dual-protocol models; feeds check_map and check_not_mapped." "Python class" {
            tags "Code"
        }
        tables_check_main = component "check_tables.py::main" "CLI [--dir] [--today] [--markdown] [STEM ...]; renders per-table flags, dual-protocol list, errors and a summary line." "Python function" {
            tags "Code"
        }
    }

    group "tables: e-Manual reader" {
        tables_em_pages = component "emanual.py::PAGES" "Table stem -> English e-Manual page URL for every table." "Python dict" {
            tags "Code"
        }
        tables_em_parse_page = component "emanual.py::parse_page" "HTML table extraction by header names; area from an Area column or the EEPROM/RAM heading; R/RW recorded as RW." "Python function" {
            tags "Code"
        }
        tables_em_compare = component "emanual.py::compare" "Row-by-row diff of a table against the page rows (backs the diff command)." "Python function" {
            tags "Code"
        }
        tables_em_fill = component "emanual.py::fill_text" "Adds access and area only to rows matching one page row on address, size and name; lists rows left for a human." "Python function" {
            tags "Code"
        }
        tables_em_main = component "emanual.py::main" "CLI: rows URL, diff STEM [URL], fill STEM [URL]; exit 2 on network or parse error." "Python function" {
            tags "Code"
        }
    }

    group "tables: Layout analyzer" {
        tables_an_main = component "analyze_tables.py::main" "Parses (address, size, data_name) triples and protocols per table, clusters layouts, writes both reports." "Python function" {
            tags "Code"
        }
        tables_an_report_md = component "control_tables/control_table_analysis.md" "Generated human report: exact-duplicate groups, register consistency, layout tiers." "Markdown, generated" {
            tags "Code"
        }
        tables_an_report_json = component "control_tables/control_table_analysis.json" "Generated machine-readable dump of the same analysis." "JSON, generated" {
            tags "Code"
        }
    }

    group "tables: Tool tests" {
        tables_test_check = component "control_tables/test_check_tables.py" "Checker tests against temp directories and the real tables." "Python unittest" {
            tags "Code"
        }
        tables_test_emanual = component "control_tables/test_emanual.py" "Reader tests on canned HTML, no network." "Python unittest" {
            tags "Code"
        }
    }

    // Code-level relationships
    tables_map_models -> tables_ax_12a "Points model 12 at"
    tables_map_models -> tables_mx_64_p1 "Points model 310 (p1) at"
    tables_map_models -> tables_mx_64 "Points model 311 (p1 and p2) at"
    tables_map_models -> tables_xm430_w350 "Points model 1020 (p1 and p2) at"
    tables_map_models -> tables_xl330_fw_variants "Points model 1190 (p2, from_fw 0 and 53) at"
    tables_map_not_mapped -> tables_ax_12_plus "Leaves out as SameNumberAs(12)"
    tables_check_main -> tables_check_load "Calls"
    tables_check_main -> tables_check_run_checks "Calls"
    tables_check_load -> tables_check_parse_ron "Parses each file with"
    tables_check_run_checks -> tables_check_map_info "Builds and queries"
    tables_check_run_checks -> tables_ron_schema "Enforces"
    tables_check_map_info -> tables_map_models "Summarises"
    tables_check_run_checks -> tables_map_not_mapped "Checks completeness of"
    tables_em_main -> tables_em_pages "Looks up the URL in"
    tables_em_main -> tables_em_parse_page "Calls"
    tables_em_main -> tables_em_compare "Calls for diff"
    tables_em_main -> tables_em_fill "Calls for fill"
    tables_em_fill -> tables_check_parse_ron "Reads the table with"
    tables_an_main -> tables_check_parse_ron "Reads protocols and the map with"
    tables_an_main -> tables_an_report_md "Writes"
    tables_an_main -> tables_an_report_json "Writes"
    tables_test_check -> tables_check_main "Tests"
    tables_test_emanual -> tables_em_parse_page "Tests"
}

// Relationships with workspace elements
maintainer -> dxl.tables.tables_files "Edits"
maintainer -> dxl.tables.tables_emanual_reader "Runs rows / diff / fill" "python3"
maintainer -> dxl.tables.tables_analyzer "Runs" "python3"
dxl.tables.tables_emanual_reader -> emanual "Fetches control-table pages from" "HTTPS, urllib"
dxl.tables.tables_em_main -> emanual "Fetches the page named in PAGES from" "HTTPS, urllib"
ci -> dxl.tables.tables_checker "Runs on every push (job control-tables)" "python3 check_tables.py"
ci -> dxl.tables.tables_tool_tests "Runs" "python3 -m unittest"
dxl.xtask -> dxl.tables.tables_map "Parses" "serde + ron"
dxl.xtask -> dxl.tables.tables_files "Parses source, protocols, operating_modes, type_info, rows" "serde + ron"

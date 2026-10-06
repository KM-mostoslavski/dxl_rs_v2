from gen import Diagram

d = Diagram()
d.title("dxl_rs (v1, previous) — class diagram", 40, 0)

# column A: lib + port + ffi
d.column(40, 100, [
    ("lib", "lib.rs", ["+ type Result<T> = core::result::Result<T, Error>", "- static INIT: Once",
                       "- const COMM_SUCCESS: u8", "---", "+ init()  (calls packet_handler once)"],
     {"stereo": "Module", "w": 330}),
    ("sdk", "dxl_sdk  (bindgen of C DynamixelSDK)", ["portHandler / openPort / setBaudRate", "ping / broadcastPing",
                                                      "read*/write*TxRx", "groupSyncRead* / groupSyncWrite*",
                                                      "getLastTxRxResult / getRxPacketError"],
     {"stereo": "FFI", "w": 330}),
    ("ph", "PortHandler<'a>", ["+ dev: &'a str  = \"/dev/ttyUSB0\"", "+ baud: i32  = 1_000_000", "---",
                               "- port_handler(CString) -> i32", "+ close(port: i32)",
                               "+ get_port_number(&self) -> Result<PortNumber>",
                               "+ open_port(&self, PortNumber) -> Result<()>", "+ open(&self) -> Result<PortNumber>"],
     {"w": 330}),
    ("pb", "PortBuilder", ["#derive_builder::Builder", "+ dev(..) / baud(..) / build()"], {"w": 330}),
    ("pn", "PortNumber = c_int", [], {"stereo": "Type alias", "w": 330}),
])

# column B: actuator traits
d.column(450, 100, [
    ("act", "Actuator", ["+ type Field: DxlField", "---", "+ id(&self) -> u8", "+ protocol(&self) -> c_int",
                         "+ port_num(&self) -> PortNumber", "+ goal_limits(&self) -> (u16, u16)",
                         "+ ping(&self) -> Result<()>  [default]", "+ model(&self) -> Model",
                         "+ goal_position(&self) -> Register", "+ read_present_position(&self) -> Result<u32>",
                         "+ register_of(&self, Readable) -> Option<Register>"], {"stereo": "Trait", "w": 360}),
    ("aoff", "ActuatorOff: Actuator + Sized", ["+ type On: ActuatorOn", "---",
                                                "+ enable_torque(self) -> Result<Self::On>"], {"stereo": "Trait", "w": 360}),
    ("aon", "ActuatorOn: Actuator + Sized", ["+ type Off: ActuatorOff", "---",
                                              "+ disable_torque(&self) -> Result<Self::Off>",
                                              "+ validate_goal_position(&self, u32) -> u32  [default]",
                                              "+ write_goal_position(&self, u32)"], {"stereo": "Trait", "w": 360}),
    ("toff", "TorqueOff", [], {"stereo": "Marker", "w": 170}),
    ("ton", "TorqueOn", [], {"stereo": "Marker", "w": 170}),
])

d.column(870, 100, [
    ("gen", "<Model><S = TorqueOff>  (x72)", ["+ id: u8", "+ port_num: PortNumber", "+ protocol: c_int",
                                               "+ goal_limits: (u16, u16)", "~ _state: PhantomData<S>", "---",
                                               "+ new(id, port_num) -> Self  [TorqueOff]",
                                               "- read_goal_limits(id, port_num) -> (u16, u16)",
                                               "#e.g. Ax12a, Mx64, Xm430W210, Xw430T200, Xw540T140 ...",
                                               "#emitted by build.rs (actuator_codegen) from docs/*.ron"],
     {"stereo": "Generated", "w": 360}),
    ("modelenum", "Model", ["#72 variants, one per .ron control table", "Ax12a, Mx64, Xm430W210, ..."],
     {"stereo": "Enum", "w": 360}),
    ("readable", "Readable", ["PresentPosition", "PresentVelocity", "PresentPwm", "PresentCurrent", "PresentLoad",
                              "PresentInputVoltage", "PresentTemperature", "Moving", "MovingStatus",
                              "HardwareErrorStatus", "RealtimeTick"], {"stereo": "Enum", "w": 220}),
    ("protos", "Protocols", ["V1", "V2", "---", "+ const value(self) -> c_int"], {"stereo": "Enum", "w": 220}),
    ("alias", "Ax12A<S> / Mx64Ar<S>", ["#ax_series / mx_series back-compat aliases"], {"stereo": "Type alias", "w": 360}),
])

# column D: dxl_config generated
d.column(1300, 100, [
    ("reg", "Register", ["+ address: u16", "+ size: u16"], {"w": 260}),
    ("dxf", "DxlField", ["+ register(&self) -> Register", "+ address(&self) -> u16  [default]",
                         "+ size(&self) -> u16  [default]"], {"stereo": "Trait", "w": 260}),
    ("field", "<model>::Field  (x72 mods)", ["Model_Number, Model_Information, ...", "---",
                                             "+ const register(self) -> Register",
                                             "+ register_by_address(u16) -> Option<Register>"],
     {"stereo": "Generated", "w": 300}),
])

# row 2: actuators_list
Y2 = 1350
d.column(40, Y2, [
    ("alist", "ActuatorsList<Proto, S = TorqueOff>", [
        "- _proto: PhantomData<Proto>", "- _state: PhantomData<S>", "- items: Vec<ActuatorVariant<S>>", "---",
        "#impl<Proto, S>", "+ new() / is_empty() / len()", "+ ids_for_port(PortNumber) -> Vec<u8>",
        "+ read_present_position(id) -> Result<u32>", "+ ping_all() -> Result<()>",
        "+ read_all_present_positions() -> Vec<(u8, Result<u32>)>", "---",
        "#impl<S> <P1, S> / <P2, S>", "+ push<T: P1Model | P2Model>(T)", "---",
        "#impl<S> <P2, S>  — SYNC READ", "+ sync_read(Readable) -> Result<Vec<(u8, Option<u32>)>>",
        "+ sync_read_present_voltages() / _positions()", "---",
        "#impl <P1|P2, TorqueOff>", "+ enable_torque(self) -> Result<ActuatorsList<_, TorqueOn>>", "---",
        "#impl <P1|P2, TorqueOn>", "+ disable_torque(self) -> Result<ActuatorsList<_, TorqueOff>>",
        "+ write_goal_position_all(u16)", "+ sync_write_goal_positions(&[(u8, u16)]) -> Result<()>",
        "+ write_goal_position(id, u16) -> Result<()>"], {"w": 440}),
    ("tea", "TorqueEnabledActuators<Proto> = ActuatorsList<Proto, TorqueOn>", [], {"stereo": "Type alias", "w": 440}),
])
d.column(540, Y2, [
    ("var", "ActuatorVariant<S = TorqueOff>", ["Ax12A(Ax12A<S>)", "Mx64Ar(Mx64Ar<S>)", "Xm430W210(..)", "Xw430T200(..)",
                                                "Xw540T140(..)", "Xw540T260(..)", "---",
                                                "+ id() / protocol() / port_num()", "+ read_present_position()",
                                                "+ goal_position() -> Register", "+ register_of(Readable)",
                                                "+ enable_torque(self)  [TorqueOff]",
                                                "+ write_goal_position(u32)  [TorqueOn]",
                                                "#impl From<Model<S>> for each model"],
     {"stereo": "Enum", "w": 330}),
    ("buckets", "ModelBuckets<S>", ["#one Vec per model (generated, unused)", "---", "+ new() / push() / is_empty()",
                                    "+ enable_torque() / disable_torque()", "+ write_goal_position_all(u32)"],
     {"stereo": "Generated", "w": 330}),
])
d.column(910, Y2, [
    ("p1", "P1", [], {"stereo": "Marker", "w": 150}),
    ("p2", "P2", [], {"stereo": "Marker", "w": 150}),
])
d.column(1090, Y2, [
    ("p1m", "P1Model", [], {"stereo": "Trait", "w": 150}),
    ("p2m", "P2Model", [], {"stereo": "Trait", "w": 150}),
])
d.column(1300, Y2 + 330, [
    ("macros", "dxl_macros::dxl_union!", ["#proc-macro: emits ActuatorVariant,", "#From impls, ModelBuckets"],
     {"stereo": "Macro", "w": 330}),
    ("mrules", "macro.rs", ["generate_actuator!", "generate_protocol1_actuator!"], {"stereo": "Macro", "w": 330}),
])

# transport + errors
d.column(1300, Y2, [
    ("tr", "transport.rs", ["+ packet_handler()", "- print_result(protocol, code) -> String",
                            "- communication_result(port, protocol) -> Result<()>",
                            "- error_packet(port, protocol) -> Result<()>",
                            "+ read_result(port, protocol) -> Result<()>",
                            "+ broadcast_ping_p1(port, &ActuatorsList<P1>) -> Result<Option<Vec<u8>>>",
                            "+ broadcast_ping_p2(port, &ActuatorsList<P2>) -> Result<Option<Vec<u8>>>",
                            "+ group_sync_read(port, addr, size, &[u8]) -> Result<Vec<(u8, Option<u32>)>>"],
     {"stereo": "Module", "w": 470}),
])
Y3 = Y2 + 820
d.column(40, Y3, [("err", "Error", ["FolderEmpty", "Port(PortError)", "Actuator(ActuatorError)",
                                     "Transport(TransportError)", "Communication(CommunicationError)",
                                     "StatusPacket(StatusPacketError)", "Io(std::io::Error)", "Cstring(NulError)",
                                     "#thiserror"], {"stereo": "Enum", "w": 280})])
d.column(380, Y3, [("perr", "PortError", ["FailedToOpenPort", "FailedToSetBaudrate"], {"stereo": "Enum", "w": 220}),
                   ("aerr", "ActuatorError", ["ExeededAngleLimit(u32, u32, u32)", "ActuatorNotFound(u8)"],
                    {"stereo": "Enum", "w": 220})])
d.column(640, Y3, [("terr", "TransportError", ["DisallowedAction", "BroadcastPingFailed", "GroupSyncWriteInitFailed",
                                               "GroupSyncWriteAddParamFailed", "GroupSyncReadAddParamFailed"],
                    {"stereo": "Enum", "w": 250})])
d.column(930, Y3, [("cerr", "CommunicationError", ["PortBusy, TxFail, RxFail, TxError", "RxWaiting, RxTimeout, RxCorrupt",
                                                   "NotAvailable, Unknown(i32)", "---",
                                                   "+ const from_comm_code(c_int) -> Result<(), Self>"],
                    {"stereo": "Enum", "w": 330})])
d.column(1300, Y3, [("serr", "StatusPacketError", ["Message(String)", "NullMessage { code: u8 }"],
                     {"stereo": "Enum", "w": 250})])

d.package("crate root / ffi / mod port", ["lib", "sdk", "ph", "pb", "pn"])
d.package("mod actuator", ["act", "aoff", "aon", "toff", "ton", "gen", "modelenum", "readable", "protos", "alias"])
d.package("mod dxl_config::generated  (build-time)", ["reg", "dxf", "field"])
d.package("mod actuators_list", ["alist", "tea", "var", "buckets", "p1", "p2", "p1m", "p2m"])
d.package("macros", ["macros", "mrules"])
d.package("mod transport", ["tr"])
d.package("mod error", ["err", "perr", "aerr", "terr", "cerr", "serr"])

d.edge("aoff", "act", "impl", "")
d.edge("aon", "act", "impl", "")
d.edge("act", "dxf", "assoc", "Field")
d.edge("dxf", "reg", "dep", "")
d.edge("field", "dxf", "impl", "")
d.edge("gen", "act", "impl", "impl<S>")
d.edge("gen", "aoff", "impl", "<TorqueOff>")
d.edge("gen", "aon", "impl", "<TorqueOn>")
d.edge("gen", "toff", "assoc", "S")
d.edge("gen", "ton", "assoc", "S")
d.edge("gen", "p1m", "impl", "")
d.edge("gen", "p2m", "impl", "")
d.edge("gen", "protos", "dep", "")
d.edge("alias", "gen", "dep", "alias")
d.edge("act", "readable", "dep", "")
d.edge("act", "modelenum", "dep", "")
d.edge("alist", "var", "comp", "items 0..*")
d.edge("alist", "p1", "assoc", "Proto")
d.edge("alist", "p2", "assoc", "Proto")
d.edge("var", "gen", "comp", "wraps")
d.edge("alist", "tr", "dep", "uses")
d.edge("macros", "var", "dep", "generates")
d.edge("macros", "buckets", "dep", "generates")
d.edge("mrules", "gen", "dep", "generates")
d.edge("ph", "pb", "dep", "builder")
d.edge("ph", "sdk", "dep", "FFI")
d.edge("tr", "sdk", "dep", "FFI")
d.edge("tr", "err", "dep", "")
d.edge("err", "perr", "comp", "")
d.edge("err", "aerr", "comp", "")
d.edge("err", "terr", "comp", "")
d.edge("err", "cerr", "comp", "")
d.edge("err", "serr", "comp", "")

d.note("Omitted: src/main.rs (2.1k lines of benchmark scaffolding: BenchIface, BenchEnum, "
       "BenchmarkStats, SystemConfig...) and logger.rs (empty).<br>"
       "72 generated model structs collapsed into one template box.", 1300, Y2 + 580, 470, 90)

open("/tmp/dxl_class_diagrams/dxl_rs_v1-class-diagram.drawio", "w").write(d.xml())

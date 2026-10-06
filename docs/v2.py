from gen import Diagram

d = Diagram()
d.title("dxl_rs_v2 — current state (class diagram)", 40, 0)

reads = ["model_number() -> Result<u16>", "model_information() -> Result<u32>", "firmware_version() -> Result<u8>",
         "id() -> Result<u8>", "baud_rate() -> Result<Baud>", "return_delay_time() -> Result<u8>",
         "drive_mode() -> Result<DriveMode>", "operating_mode() -> Result<OperatingMode>", "secondary_id() -> Result<u8>",
         "protocol_type() -> Result<Protocol>", "homing_offset() -> Result<i32>", "moving_threshold() -> Result<u32>",
         "temperature_limit() -> Result<u8>", "max_voltage_limit() -> Result<u16>", "min_voltage_limit() -> Result<u16>",
         "pwm_limit() -> Result<u16>", "current_limit() -> Result<u16>", "velocity_limit() -> Result<u32>",
         "max_position_limit() -> Result<u32>", "min_position_limit() -> Result<u32>", "shutdown() -> Result<Shutdown>",
         "- present_position() -> Result<u32>"]
writes = ["set_id(u8)", "set_baud_rate(Baud)", "set_return_delay_time(u8)", "set_drive_mode(DriveMode)",
          "set_operating_mode(OperatingMode)", "set_secondary_id(u8)", "set_protocol_type(Protocol)",
          "set_homing_offset(i32)", "set_moving_threshold(u32)", "set_temperature_limit(u8)",
          "set_max_voltage_limit(u16)", "set_min_voltage_limit(u16)", "set_pwm_limit(u16)", "set_current_limit(u16)",
          "set_velocity_limit(u32)", "set_max_position_limit(u32)", "set_min_position_limit(u32)",
          "set_shutdown(Shutdown)"]

d.column(40, 100, [
    ("dyn", "Dynamixel<S>", [
        "- model: Model", "- id: u8", "- baud_rate: Baud", "- protocol: Protocol",
        "- operating_mode: OperatingMode", "- _torque_state: PhantomData<S>", "---",
        "#impl<S> — reads (all todo!())"] + ["+ " + r if not r.startswith("-") else r for r in reads] +
        ["---", "#impl Dynamixel<TorqueOff> — EEPROM writes"] + ["+ " + w + " -> Result<()>" for w in writes] +
        ["+ enable_torque(self) -> Result<Dynamixel<TorqueOn>>", "---", "#impl Dynamixel<TorqueOn>",
         "- goal_position(&mut self, u32) -> Result<()>", "- disable_torque(self) -> Result<Dynamixel<TorqueOff>>"],
     {"w": 380}),
])

d.column(560, 100, [
    ("toff", "TorqueOff", [], {"stereo": "Marker"}),
    ("ton", "TorqueOn", [], {"stereo": "Marker"}),
    ("model", "Model", ["Rx28", "Mx64", "Ax12", "---", "+ number(self) -> u16"], {"stereo": "Enum"}),
    ("proto", "Protocol", ["Protocol1", "Protocol2"], {"stereo": "Enum"}),
    ("opmode", "OperatingMode", ["Current", "Velocity", "Position  (default)", "ExtendedPosition",
                                 "CurrentBasedPosition", "Pwm"], {"stereo": "Enum"}),
    ("bus", "Bus", ["Ttl", "Rs485"], {"stereo": "Enum"}),
], w=240)

flag_methods = ["+ const from_bits(u8) -> Self", "+ const bits(self) -> u8", "+ const contains(self, Self) -> bool",
                "+ const union(self, Self) -> Self", "+ const difference(self, Self) -> Self"]
d.column(870, 100, [
    ("drive", "DriveMode", ["- 0: u8", "---", "+ REVERSE = 1&lt;&lt;0".replace("&lt;", "<"), "+ SLAVE = 1<<1",
                            "+ TIME_PROFILE = 1<<2", "+ TORQUE_ON_BY_GOAL = 1<<3", "+ NORMAL = 0", "---"] + flag_methods +
     ["+ const is_reverse(self) -> bool", "+ const is_slave(self) -> bool", "+ const is_time_profile(self) -> bool",
      "---", "#impl BitOr, BitOrAssign"], {"w": 290}),
    ("shut", "Shutdown", ["- 0: u8", "---", "+ INPUT_VOLTAGE = 1<<0", "+ OVERHEATING = 1<<2", "+ MOTOR_ENCODER = 1<<3",
                          "+ ELECTRICAL_SHOCK = 1<<4", "+ OVERLOAD = 1<<5", "+ DEFAULT = 0x34", "+ NONE = 0", "---"] +
     flag_methods + ["---", "#impl BitOr, BitOrAssign"], {"w": 290}),
])

d.column(1270, 100, [
    ("usb", "UsbPortHandler", ["- available_ports: Vec<Candidate>", "---",
                               "- extract_candidates(Vec<(PortPath, UsbPortInfo)>) -> Vec<Candidate>",
                               "- find_candidates() -> Vec<Candidate>", "+ discover() -> Self",
                               "+ candidates(&self) -> &[Candidate]", "+ open_all(&self) -> Vec<Box<dyn SerialPort>>"],
     {"w": 420}),
    ("cand", "Candidate", ["+ path: PortPath", "+ serial: String"]),
    ("baud", "Baud", ["Bps9_600  (default)", "Bps57_600", "Bps115_200", "Bps1_000_000", "Bps2_000_000",
                      "Bps3_000_000", "Bps4_000_000", "Bps4_500_000", "---", "+ as_u32(&self) -> u32"], {"stereo": "Enum"}),
    ("pp", "PortPath = String", [], {"stereo": "Type alias"}),
], w=260)

d.column(1800, 100, [
    ("sp", "serialport::SerialPort", ["#(crate serialport 4.9)"], {"stereo": "External", "w": 240}),
])
d.column(1800, 260, [("main", "main.rs", ["main(): discover() + print candidates"], {"stereo": "Module", "w": 280})])

d.package("mod actuator  (lib.rs, inline)", ["dyn", "toff", "ton", "model", "proto", "opmode", "bus", "drive", "shut"])
d.package("mod port_handler  (port_handler.rs)", ["usb", "cand", "baud", "pp"])
d.package("bin", ["main"])

d.edge("dyn", "model", "comp", "model")
d.edge("dyn", "proto", "comp", "protocol")
d.edge("dyn", "opmode", "comp", "operating_mode")
d.edge("dyn", "baud", "comp", "baud_rate")
d.edge("dyn", "toff", "assoc", "S = TorqueOff")
d.edge("dyn", "ton", "assoc", "S = TorqueOn")
d.edge("dyn", "drive", "dep", "uses")
d.edge("dyn", "shut", "dep", "uses")
d.edge("usb", "cand", "comp", "0..*")
d.edge("cand", "pp", "dep", "")
d.edge("usb", "baud", "dep", "default()")
d.edge("usb", "sp", "dep", "opens")
d.edge("main", "usb", "dep", "uses")

d.note("Not yet used: <b>Bus</b> enum, <b>src/command.rs</b> (empty), <b>thiserror</b> dependency.<br>"
       "All Dynamixel methods are <i>todo!()</i>; no transport / packet layer yet.<br>"
       "control_tables/*.ron present but not wired into the build.", 1800, 420, 300, 110)

open("/tmp/dxl_class_diagrams/dxl_rs_v2-class-diagram.drawio", "w").write(d.xml())

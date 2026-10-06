use core::time::Duration;

use serialport::{SerialPort, SerialPortType, UsbPortInfo, available_ports};

pub type PortPath = String;

#[derive(Debug, PartialEq)]
pub struct Candidate {
    pub path: PortPath,
    pub serial: String,
}

pub struct UsbPortHandler {
    available_ports: Vec<Candidate>,
}

#[derive(Default)]
pub enum Baud {
    #[default]
    Bps9_600,
    Bps57_600,
    Bps115_200,
    Bps1_000_000,
    Bps2_000_000,
    Bps3_000_000,
    Bps4_000_000,
    Bps4_500_000,
}

impl Baud {
    pub fn as_u32(&self) -> u32 {
        match self {
            Baud::Bps1_000_000 => 1_000_000,
            Baud::Bps9_600 => 9_600,
            Baud::Bps57_600 => 57_600,
            Baud::Bps115_200 => 115_200,
            Baud::Bps2_000_000 => 2_000_000,
            Baud::Bps3_000_000 => 3_000_000,
            Baud::Bps4_000_000 => 4_000_000,
            Baud::Bps4_500_000 => 4_500_000,
        }
    }
}

impl UsbPortHandler {
    fn extract_candidates(ports: Vec<(PortPath, UsbPortInfo)>) -> Vec<Candidate> {
        ports
            .into_iter()
            .filter_map(|(path, info)| info.serial_number.map(|serial| Candidate { path, serial }))
            .collect()
    }

    fn find_candidates() -> Vec<Candidate> {
        let ports = available_ports()
            .expect("No ports found!")
            .into_iter()
            .filter_map(|p| match p.port_type {
                SerialPortType::UsbPort(info) => Some((p.port_name, info)),
                _ => None,
            })
            .collect();

        Self::extract_candidates(ports)
    }

    /// Discovers all USB candidates. Only way to build a handler, so
    /// `available_ports` is always populated.
    pub fn discover() -> Self {
        Self {
            available_ports: Self::find_candidates(),
        }
    }

    /// Candidates discovered at construction.
    pub fn candidates(&self) -> &[Candidate] {
        &self.available_ports
    }

    /// Opens all discovered USB ports with the default baud rate (1 Mbps)
    /// and returns them all as a vector.
    pub fn open_all(&self) -> Vec<Box<dyn SerialPort>> {
        let mut v = Vec::new();

        for a in &self.available_ports {
            let port = serialport::new(&a.path, Baud::default().as_u32())
                .timeout(Duration::from_millis(10))
                .open()
                .expect("Failed to open port");
            v.push(port);
        }

        v
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serialport::UsbPortInfo;

    fn fake_usb_info(serial: Option<&str>) -> UsbPortInfo {
        UsbPortInfo {
            vid: 0x0483,
            pid: 0x5740,
            serial_number: serial.map(|s| s.to_string()),
            manufacturer: None,
            product: None,
        }
    }

    #[test]
    fn extracts_serials_when_present() {
        let ports = vec![
            ("/dev/ttyUSB0".to_string(), fake_usb_info(Some("AL02L1KL"))),
            ("/dev/ttyUSB1".to_string(), fake_usb_info(None)),
            ("/dev/ttyUSB2".to_string(), fake_usb_info(Some("XYZ123"))),
        ];

        let result = UsbPortHandler::extract_candidates(ports);

        assert_eq!(
            result,
            vec![
                Candidate {
                    path: "/dev/ttyUSB0".to_string(),
                    serial: "AL02L1KL".to_string(),
                },
                Candidate {
                    path: "/dev/ttyUSB2".to_string(),
                    serial: "XYZ123".to_string(),
                },
            ]
        );
    }

    #[test]
    fn returns_empty_when_no_serials() {
        let ports = vec![
            ("/dev/ttyUSB0".to_string(), fake_usb_info(None)),
            ("/dev/ttyUSB1".to_string(), fake_usb_info(None)),
        ];
        let result = UsbPortHandler::extract_candidates(ports);
        assert!(result.is_empty());
    }
}

use dxl_rs_v2::port_handler::UsbPortHandler;

fn main() {
    let handler = UsbPortHandler::discover();

    for c in handler.candidates() {
        println!("{} -> {}", c.path, c.serial);
    }
}

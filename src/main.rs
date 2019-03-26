mod i8080;
mod machine;

use crate::machine::Machine;
use machine::SpaceInvaders;
use std::time::Duration;

fn main() {
    // Init machine
    let mut machine = SpaceInvaders::new();

    // Create window
    let mut window = minifb::Window::new(
        "rust-8080",
        machine.width(),
        machine.height(),
        minifb::WindowOptions {
            borderless: false,
            title: true,
            resize: false,
            scale: minifb::Scale::X2,
        },
    )
    .unwrap();

    while window.is_open() {
        // Start of frame interrupt
        machine.interrupt(1);
        std::thread::sleep(Duration::from_millis(8));

        machine.step(0.008);

        window.update_with_buffer(&machine.screen()).unwrap();

        // VBlank interrupt
        machine.interrupt(2);
        std::thread::sleep(Duration::from_millis(8));

        machine.step(0.008);
    }
}

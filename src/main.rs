#![deny(clippy::pedantic)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use invaders::SpaceInvaders;

mod cpu;
mod invaders;
mod flags;

fn main() {
    // Init machine
    let mut invaders = SpaceInvaders::new();

    // Create window
    let mut window = minifb::Window::new(
        "rust-8080",
        SpaceInvaders::SCREEN_WIDTH,
        SpaceInvaders::SCREEN_HEIGHT,
        minifb::WindowOptions {
            borderless: false,
            title: true,
            resize: false,
            scale: minifb::Scale::X2,
        },
    ).expect("Could not create window");

    while window.is_open() {
        invaders.step(&mut window);
    }
}

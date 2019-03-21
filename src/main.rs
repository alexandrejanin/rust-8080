extern crate piston;
extern crate piston_window;

use piston_window::{self as window, Transformed};
use std::time::Instant;

mod i8080;

fn main() {
    let mut window: window::PistonWindow =
        window::WindowSettings::new("Space Invaders", (224, 256))
            .vsync(true)
            .build()
            .unwrap_or_else(|e| panic!("Failed to create window: {}", e));

    let mut glyphs = window::Glyphs::new(
        "RobotoMono-Regular.ttf",
        window.factory.clone(),
        window::TextureSettings::new(),
    ).unwrap_or_else(|e| panic!("Failed to create font: {}", e));

    let rom = include_bytes!("invaders.rom");

    let mut state = i8080::State8080::new(rom);

    let mut cycles = 0;

    let mut last_instant = Instant::now();

    // While window is open
    while let Some(e) = window.next() {
        let now = Instant::now();

        // Get next state
        cycles += state.step((now - last_instant).as_nanos());

        last_instant = now;

        // Draw window
        window.draw_2d(&e, |c, g| {
            window::clear([0.5, 1.0, 0.5, 1.0], g);

            let x = 10.0;
            let mut y = 22.0;

            // Display state information
            for line in &[
                format!("cycle: {}", cycles),
                state.next_opcode(),
                format!("AF: {:04x}", state.af()),
                format!("BC: {:04x}", state.bc()),
                format!("DE: {:04x}", state.de()),
                format!("HL: {:04x}", state.hl()),
                format!("PC: {:04x}", state.pc()),
                format!("SP: {:04x}", state.sp()),
            ] {
                window::text(
                    window::color::BLACK,
                    12,
                    &line,
                    &mut glyphs,
                    c.transform.trans(x, y),
                    g,
                )
                    .unwrap();
                y += 15.0;
            }
        });
    }
}

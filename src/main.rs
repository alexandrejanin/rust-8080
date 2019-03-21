extern crate piston_window;

use std::time::Instant;
use piston_window::*;

mod i8080;

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Space Invaders", (224, 256))
        //.vsync(true)
        .build()
        .unwrap_or_else(|e| panic!("Failed to create window: {}", e));

    let mut glyphs = Glyphs::new("RobotoMono-Regular.ttf", window.factory.clone(), TextureSettings::new())
        .unwrap_or_else(|e| panic!("Failed to create font: {}", e));


    let rom = include_bytes!("invaders.rom");

    let mut state = i8080::State8080::new(rom);

    let mut i = 0;

    // While window is open
    while let Some(e) = window.next() {
        i += 1;

        // Get next state
        state.emulate();

        let x = 10.0;
        let mut y = 22.0;

        // Draw window
        window.draw_2d(&e, |c, g| {
            piston_window::clear([0.5, 1.0, 0.5, 1.0], g);

            piston_window::text(
                piston_window::color::BLACK, 12,
                &format!("{}", i),
                &mut glyphs, c.transform.trans(x, y), g
            ).unwrap();
            y += 15.0;

            piston_window::text(
                piston_window::color::BLACK, 12,
                &state.next_opcode(),
                &mut glyphs, c.transform.trans(x, y), g
            ).unwrap();
            y += 15.0;

            piston_window::text(
                piston_window::color::BLACK, 12,
                &format!("AF: {:04x}", state.af()),
                &mut glyphs, c.transform.trans(x, y), g
            ).unwrap();
            y += 15.0;

            piston_window::text(
                piston_window::color::BLACK, 12,
                &format!("BC: {:04x}", state.bc()),
                &mut glyphs, c.transform.trans(x, y), g
            ).unwrap();
            y += 15.0;

            piston_window::text(
                piston_window::color::BLACK, 12,
                &format!("DE: {:04x}", state.de()),
                &mut glyphs, c.transform.trans(x, y), g
            ).unwrap();
            y += 15.0;

            piston_window::text(
                piston_window::color::BLACK, 12,
                &format!("HL: {:04x}", state.hl()),
                &mut glyphs, c.transform.trans(x, y), g
            ).unwrap();
            y += 15.0;

            piston_window::text(
                piston_window::color::BLACK, 12,
                &format!("PC: {:04x}", state.pc()),
                &mut glyphs, c.transform.trans(x, y), g
            ).unwrap();
            y += 15.0;

            piston_window::text(
                piston_window::color::BLACK, 12,
                &format!("SP: {:04x}", state.sp()),
                &mut glyphs, c.transform.trans(x, y), g
            ).unwrap();
            y += 15.0;
        });
    }
}

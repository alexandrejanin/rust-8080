use piston_window::{
    self as pw, clear, color, image, text, Event, EventLoop, Glyphs, Input, Loop, Texture,
    TextureSettings, Transformed, WindowSettings
};

mod i8080;
mod machine;

use crate::machine::Machine;
use machine::SpaceInvaders;
use std::time::Duration;

fn main() {
    // Init machine
    let mut machine = SpaceInvaders::new();

    // Create window
    let mut window: pw::PistonWindow =
        WindowSettings::new("Space Invaders", (2 * machine.width(), machine.height()))
            .resizable(false)
            .vsync(false)
            .build()
            .unwrap_or_else(|e| panic!("Failed to create window: {}", e));

    window = window.ups(60).max_fps(60);

    // Load font
    let mut glyphs = Glyphs::new(
        "RobotoMono-Regular.ttf",
        window.factory.clone(),
        TextureSettings::new(),
    )
        .unwrap_or_else(|e| panic!("Failed to create font: {}", e));

    // Init screen
    let mut screen_texture = Texture::from_image(
        &mut window.factory,
        &machine.screen(),
        &TextureSettings::new(),
    )
        .unwrap_or_else(|e| panic!("Failed to create texture: {}", e));

    // While window is open
    while let Some(event) = window.next() {
        match event {
            Event::Input(input_event) => if let Input::Button(args) = input_event {
                machine.key_press(args.button, args.state)
            }
            Event::Loop(loop_event) => match loop_event {
                Loop::Update(args) => machine.step(args.dt),
                Loop::Render(_) => {
                    // Start of frame interrupt
                    //machine.interrupt(1);
                    //std::thread::sleep(Duration::from_millis(8));

                    // Convert image buffer to texture and draw
                    screen_texture
                        .update(&mut window.encoder, &machine.screen())
                        .unwrap();

                    // Draw window
                    window.draw_2d(&event, |c, g| {
                        clear([0.5, 1.0, 0.5, 1.0], g);

                        let x = 10.0;
                        let mut y = 22.0;

                        // Display state information
                        for line in machine.debug_text() {
                            text(
                                color::BLACK,
                                12,
                                &line,
                                &mut glyphs,
                                c.transform.trans(x, y),
                                g,
                            )
                                .unwrap();
                            y += 15.0;
                        }

                        image(
                            &screen_texture,
                            c.transform.trans(f64::from(machine.width()), 0.0),
                            g,
                        );
                    });

                    // VBlank interrupt
                    machine.interrupt(2);
                }
                _ => {}
            }
            _ => {}
        }
    }
}

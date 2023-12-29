#![feature(array_chunks)]

use std::rc::Rc;

use fleck::Font;
use pixels::wgpu::BlendState;
use pixels::{PixelsBuilder, SurfaceTexture};
use stammer::elements::{Alignment, WrappedText};
use stammer::elements::{Content, Element};
use stammer::Panel;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit_input_helper::{TextChar, WinitInputHelper};

const WINDOW_NAME: &str = env!("CARGO_BIN_NAME");

const LOREM: &str = include_str!("lorem.txt");
const SCROLL_STEP: usize = 8;

fn setup_window(min_size: PhysicalSize<u32>, event_loop: &EventLoop<()>) -> Window {
    let builder = WindowBuilder::new()
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(true)
        .with_title(WINDOW_NAME)
        .with_inner_size(min_size)
        .with_min_inner_size(min_size);

    builder.build(event_loop).expect("could not build window")
}

fn load_font(path: &str) -> std::io::Result<Font> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    assert_eq!(buf.len(), fleck::FILE_SIZE);
    let font = Font::new(buf.as_slice().try_into().unwrap());
    Ok(font)
}

fn setup_elements(font: Rc<Font>) -> Element<Data> {
    fn display_address(element: &mut Element<Data>, data: &Data) {
        // TODO: This whole practice is a mess and is horrible and oh no.
        let Content::Text(text, _) = &mut element.content else {
            unreachable!()
        };
        text.clear();
        text.push_str(data.address.as_str())
    }

    // FIXME: Implement ability to scroll containers for this to work.
    // fn update_scroll(element: &mut Element<Data>, data: &Data) {
    //     // TODO: This whole practice is a mess and is horrible and oh no.
    //     let Content::Scroll(_, _, pos) = element else {
    //         unreachable!()
    //     };
    //     *pos = data.scroll_pos;
    // }

    fn display_text(element: &mut Element<Data>, data: &Data) {
        // TODO: This whole practice is a mess and is horrible and oh no.
        let Content::Paragraph(text, _) = &mut element.content else {
            unreachable!()
        };
        *text = WrappedText::new(data.text.clone(), data.width, &element.style.font)
    }

    fn display_mode(element: &mut Element<Data>, data: &Data) {
        // TODO: This whole practice is a mess and is horrible and oh no.
        let Content::Text(text, _) = &mut element.content else {
            unreachable!()
        };
        text.clear();
        text.push_str(data.mode.to_string().as_str())
    }

    {
        use Content::*;
        Element::still(
            Rc::clone(&font),
            Stack(vec![
                Element::dynamic(
                    display_address,
                    Rc::clone(&font),
                    Text("---".to_string(), Alignment::Left),
                ),
                // FIXME: Implement ability to scroll containers for this to work.
                // Element::dynamic(
                //     update_scroll,Rc::clone(&font),
                //     Scroll(
                //         Box::new(
                //             Element::dynamic(display_text,Rc::clone(&font),  Paragraph(WrappedText::default()))
                //         ),
                //         300,
                //         0,
                //     ),
                // ),
                Element::dynamic(
                    display_mode,
                    Rc::clone(&font),
                    Text("---".to_string(), Alignment::Left),
                ),
            ]),
        )
    }
}

struct Data {
    text: String,
    scroll_pos: usize,
    address: String,
    mode: Mode,
    width: u32,
}

#[derive(PartialEq, Eq)]
enum Mode {
    Normal,
    Insert,
    Link,
}

impl ToString for Mode {
    fn to_string(&self) -> String {
        match self {
            Mode::Normal => "normal".to_string(),
            Mode::Insert => "insert".to_string(),
            Mode::Link => "link".to_string(),
        }
    }
}

fn main() -> Result<(), pixels::Error> {
    todo!(
        "This example is currently utterly broken.
This repo is in a state of transition.
    (there's a joke here somewhere about rust programmers)
Please feel free to hack at the functionality that is currently broken!"
    );

    let mut args = std::env::args().skip(1);
    let font_path = args
        .next()
        .unwrap_or("/etc/tid/fonts/geneva14.uf2".to_string());
    let font = match load_font(&font_path) {
        Ok(font) => font,
        Err(err) => {
            eprintln!("ERROR: Failed to load font from {font_path:?}: {err}");
            std::process::exit(1);
        }
    };
    let font = Rc::new(font);

    let event_loop = EventLoop::new();

    let scale_factor = std::env::var("TID_SCALE_FACTOR")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .map(|v| v.round() as u32)
        .unwrap_or(1);

    let elements = setup_elements(font);
    let data = Data {
        text: [LOREM; 3].concat().to_string(),
        scroll_pos: 0,
        address: "gemini://example.com/".to_string(),
        mode: Mode::Normal,
        width: 0,
    };
    let mut state = Panel::new(
        elements,
        [0x00, 0x00, 0x00, 0xff],
        [0xff, 0xff, 0xff, 0xff],
        data,
    );

    let (width, height) = (state.width, state.height);
    let size = PhysicalSize::new(width * scale_factor, height * scale_factor);

    let mut input = WinitInputHelper::new();
    let window = setup_window(size, &event_loop);

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        PixelsBuilder::new(width, height, surface_texture)
            .clear_color({
                let [r, g, b, a] = state.background.map(|v| v as f64 / u8::MAX as f64);
                pixels::wgpu::Color { r, g, b, a }
            })
            .blend_state(BlendState::REPLACE) // TODO: Investigate rendering weirdness.
            .build()?
    };

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();

        match event {
            // Event::NewEvents(winit::event::StartCause::ResumeTimeReached { .. }) => {
            //     window.request_redraw()
            // }
            Event::RedrawRequested(_) => {
                // Clear the screen before drawing.
                pixels
                    .frame_mut()
                    .array_chunks_mut()
                    .for_each(|px| *px = state.background);

                eprintln!("INFO: Redrawing...");
                // Update the state, then draw.
                state.update();
                state.draw(&mut pixels.frame_mut());

                // Try to render.
                if let Err(err) = pixels.render() {
                    eprintln!("ERROR: {err}");
                    control_flow.set_exit();
                    return;
                }
            }
            _ => (),
        }

        if input.update(&event) {
            // Scroll around.
            if input.key_pressed(VirtualKeyCode::Up) | input.key_pressed(VirtualKeyCode::K) {
                let pos = &mut state.data_mut().scroll_pos;
                *pos = pos.saturating_sub(SCROLL_STEP);
                window.request_redraw();
            }

            if input.key_pressed(VirtualKeyCode::Down) | input.key_pressed(VirtualKeyCode::J) {
                state.data_mut().scroll_pos += SCROLL_STEP;
                window.request_redraw();
            }

            // Set mode.
            {
                let data = state.data_mut();
                let mode = &mut data.mode;

                match mode {
                    Mode::Normal => {
                        if input.key_pressed(VirtualKeyCode::I) {
                            *mode = Mode::Insert;
                            window.request_redraw();
                        }
                        if input.key_pressed(VirtualKeyCode::F) {
                            eprintln!(
                                "TODO: The implementation of `Mode::Link` has been \
                                left as an exercise to cute ppl. <3"
                            );
                            *mode = Mode::Link;
                            window.request_redraw();
                        }
                    }
                    Mode::Insert => {
                        for ch in input.text() {
                            match ch {
                                TextChar::Char('\n') => {
                                    data.address.clear();
                                    eprintln!("Please pretend some other site's text is loading.")
                                }
                                TextChar::Char(ch) => data.address.push(ch),
                                TextChar::Back => {
                                    let _ = data.address.pop();
                                }
                            }
                            window.request_redraw();
                        }
                    }
                    Mode::Link => { /* TODO */ }
                }

                if input.key_pressed(VirtualKeyCode::Escape) {
                    *mode = Mode::Normal;
                    window.request_redraw();
                }
            }

            // Close events.
            if input.close_requested() {
                eprintln!("INFO:  Close requested. Bye :)");
                control_flow.set_exit();
                return;
            }

            // Resize the window.
            if let Some(size) = input.window_resized() {
                eprintln!("INFO:  Resize request {size:?}");
                let ps = PhysicalSize {
                    width: (size.width / scale_factor) * scale_factor,
                    height: (size.height / scale_factor) * scale_factor,
                };
                let ls = LogicalSize {
                    width: ps.width / scale_factor,
                    height: ps.height / scale_factor,
                };
                pixels.resize_surface(ps.width, ps.height).unwrap();
                pixels.resize_buffer(ls.width, ls.height).unwrap();
                window.set_inner_size(ps);
                state.resize(ls.width, ls.height);
                state.data_mut().width = ls.width;

                window.request_redraw();
            }
        }
    });
}

#![feature(array_chunks)]

use std::collections::VecDeque;

use fleck::Font;
use pixels::wgpu::BlendState;
use pixels::{PixelsBuilder, SurfaceTexture};
use stammer::elements::{Element, ElementKind, Graph};
use stammer::Raam;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::Event;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit_input_helper::WinitInputHelper;

const WINDOW_NAME: &str = env!("CARGO_BIN_NAME");

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

fn setup_elements() -> Element<Data> {
    let graph_width = 150;
    let sine = stammer::elements::Graph::from({
        // A lil sine wave.
        let mut deque = VecDeque::new();
        for x in 0..graph_width {
            let y = f32::sin(x as f32 / (graph_width / 4) as f32 * std::f32::consts::FRAC_PI_2);
            deque.push_front(y)
        }
        deque
    });
    let triangle = Graph::from({
        // A triangle wave.
        let mut deque = VecDeque::new();
        for x in 0..graph_width {
            let a = graph_width / 5;
            let y = i32::abs(x % 30 - a / 2);
            deque.push_front(y as f32)
        }
        deque
    });

    fn step_graph(thing: &mut ElementKind<Data>, data: &Data) {
        // TODO: This whole practice is a mess and is horrible and oh no.
        let ElementKind::Graph(graph) = thing else {
            unreachable!()
        };
        graph.inner_mut().rotate_left(data.rotate_step);
    }

    fn display_step(thing: &mut ElementKind<Data>, data: &Data) {
        // TODO: This whole practice is a mess and is horrible and oh no.
        let ElementKind::Text(text) = thing else {
            unreachable!()
        };
        text.clear();
        text.push_str(format!("{} femtoseconds", data.rotate_step).as_str())
    }

    {
        use ElementKind::*;
        Element::still(Stack(vec![
            Element::still(Row(vec![
                Element::still(Text("measurement interval:".to_string())),
                Element::still(Space),
                Element::still(Space),
                Element::dynamic(display_step, Text("---".to_string())),
            ])),
            Element::still(Space),
            Element::still(Row(vec![
                Element::still(Stack(vec![
                    Element::still(Text("deflection coil phase".to_string())),
                    Element::still(Space),
                    Element::still(Text("tri-axial wave converter".to_string())),
                ])),
                Element::still(Space),
                Element::still(Stack(vec![
                    Element::dynamic(step_graph, Graph(sine)),
                    Element::still(Space),
                    Element::dynamic(step_graph, Graph(triangle)),
                ])),
            ])),
        ]))
    }
}

struct Data {
    rotate_step: usize,
}

fn main() -> Result<(), pixels::Error> {
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

    let event_loop = EventLoop::new();

    let scale_factor = std::env::var("TID_SCALE_FACTOR")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .map(|v| v.round() as u32)
        .unwrap_or(1);

    let elements = setup_elements();

    let data = Data { rotate_step: 1 };
    let mut state = Raam::new(
        elements,
        Box::new(font),
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

                // Update the state, then draw.
                state.update();
                state.draw(&mut pixels);

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
            if input.key_pressed(winit::event::VirtualKeyCode::Up) {
                state.data_mut().rotate_step += 1;
            }

            if input.key_pressed(winit::event::VirtualKeyCode::Down) {
                let step = &mut state.data_mut().rotate_step;
                *step = step.saturating_sub(1);
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
                let ps = LogicalSize {
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
            }

            window.request_redraw();
        }
    });
}

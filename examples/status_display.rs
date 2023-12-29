#![feature(array_chunks)]

use std::collections::VecDeque;
use std::rc::Rc;

use fleck::Font;
use pixels::wgpu::BlendState;
use pixels::{PixelsBuilder, SurfaceTexture};
use stammer::elements::{Alignment, Content, Element};
use stammer::elements::{Graph, SizingStrategy};
use stammer::Panel;
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

fn setup_elements(font: Rc<Font>) -> Element<Data> {
    let graph_width = 150;
    let sine = Graph::from({
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

    // FIXME: Reintroduce Content::Graph or something like that. This will need some thought so
    // please communicate before embarking on this. Could be in large part just taking the old
    // implementation, which was perfectly fine. But the thinking will be in how it fits with the
    // rest of the Elements.
    // fn step_graph(element: &mut Element<Data>, data: &Data) {
    //     // TODO: This whole practice is a mess and is horrible and oh no.
    //     let Content::Graph(graph) = &mut element.content else {
    //         unreachable!()
    //     };
    //     graph.inner_mut().rotate_left(data.rotate_step);
    // }

    fn display_step(element: &mut Element<Data>, data: &Data) {
        // TODO: This whole practice is a mess and is horrible and oh no.
        let Content::Text(text, _) = &mut element.content else {
            unreachable!()
        };
        text.clear();
        text.push_str(format!("{} femtoseconds", data.rotate_step).as_str())
    }

    {
        use Content::*;
        Element::still(
            Rc::clone(&font),
            Stack(vec![
                Element::still(
                    Rc::clone(&font),
                    Row(vec![
                        Element::still(
                            Rc::clone(&font),
                            Text("measurement interval:".to_string(), Alignment::Left),
                        )
                        .with_padding_right(16)
                        .with_flex_right(true),
                        Element::dynamic(
                            display_step,
                            Rc::clone(&font),
                            Text("---".to_string(), Alignment::Center),
                        ),
                    ]),
                )
                .with_minwidth(400)
                .with_strategy(SizingStrategy::Chonker)
                .with_padding_bottom(16),
                Element::still(
                    Rc::clone(&font),
                    Row(vec![
                        Element::still(
                            Rc::clone(&font),
                            Stack(vec![
                                Element::still(
                                    Rc::clone(&font),
                                    Text("deflection coil phase".to_string(), Alignment::Left),
                                )
                                .with_padding_bottom(16),
                                Element::still(
                                    Rc::clone(&font),
                                    Text("tri-axial wave converter".to_string(), Alignment::Left),
                                ),
                            ]),
                        )
                        .with_flex_right(true),
                        Element::still(
                            Rc::clone(&font),
                            Stack(vec![
                                Element::still(
                                    Rc::clone(&font),
                                    Text("TODO: Graph placeholder.".to_string(), Alignment::Right),
                                )
                                .with_padding_bottom(16),
                                Element::still(
                                    Rc::clone(&font),
                                    Text("TODO: Graph placeholder.".to_string(), Alignment::Right),
                                ),
                                // Element::dynamic(step_graph, Rc::clone(&font), Graph(sine)),
                                // Element::dynamic(step_graph, Rc::clone(&font), Graph(triangle)),
                            ]),
                        ),
                    ]),
                )
                .with_minwidth(400)
                .with_strategy(SizingStrategy::Chonker),
            ]),
        )
        .with_strategy(SizingStrategy::Chonker)
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

    let elements = setup_elements(Rc::new(font));

    let data = Data { rotate_step: 1 };
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

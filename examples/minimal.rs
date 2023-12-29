#![feature(array_chunks)]

use std::rc::Rc;

use fleck::Font;
use pixels::wgpu::BlendState;
use pixels::{PixelsBuilder, SurfaceTexture};
use stammer::elements::{Alignment, Content, Element, SizingStrategy};
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

fn setup_elements(font: Rc<Font>, cute_font: Rc<Font>) -> Element<Data> {
    fn resize_width(element: &mut Element<Data>, data: &Data) {
        element.size.maxwidth = Some(data.width);
    }

    fn resize_height(element: &mut Element<Data>, data: &Data) {
        element.size.maxheight = Some(data.height);
    }

    use Content::*;
    Element::dynamic(
        resize_height,
        Rc::clone(&font),
        Stack(vec![
            Element::dynamic(
                resize_width,
                Rc::clone(&font),
                Row(vec![
                    Element::still(
                        Rc::clone(&font),
                        Text("top left".to_string(), Alignment::Right),
                    )
                    .with_padding_top(10)
                    .with_padding_right(20)
                    .with_minwidth(150)
                    .with_background([0xff, 0xaa, 0xaa, 0xff])
                    .with_flex_right(true),
                    Element::still(
                        Rc::clone(&font),
                        Text("top right".to_string(), Alignment::Right),
                    )
                    .with_minheight(100)
                    .with_background([0xff, 0xaa, 0xff, 0xff]),
                ]),
            )
            .with_strategy(SizingStrategy::Chonker),
            Element::dynamic(
                resize_width,
                Rc::clone(&font),
                Row(vec![Element::still(
                    Rc::clone(&cute_font),
                    Text("weird flex but ok".to_string(), Alignment::Right),
                )
                .with_flex_left(true)
                .with_flex_right(true)
                .with_strategy(SizingStrategy::Chonker)]),
            )
            .with_flex_top(true)
            .with_flex_bottom(true)
            .with_strategy(SizingStrategy::Chonker),
            Element::dynamic(
                resize_width,
                Rc::clone(&font),
                Row(vec![
                    Element::still(
                        Rc::clone(&font),
                        Text("bottom left".to_string(), Alignment::Left),
                    )
                    .with_padding_left(30)
                    .with_padding_right(40)
                    .with_background([0xaa, 0xff, 0xaa, 0xff])
                    .with_flex_right(true),
                    Element::still(
                        Rc::clone(&font),
                        Text("bottom right".to_string(), Alignment::Center),
                    )
                    .with_minwidth(200)
                    .with_background([0xaa, 0xaa, 0xff, 0xff]),
                ]),
            )
            .with_strategy(SizingStrategy::Chonker),
        ]),
    )
    .with_strategy(SizingStrategy::Chonker)
}

struct Data {
    width: u32,
    height: u32,
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
    let cute_font = load_font("/etc/tid/fonts/sapphire14.uf2").expect("failed to load cute font");

    let event_loop = EventLoop::new();

    let scale_factor = std::env::var("TID_SCALE_FACTOR")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .map(|v| v.round() as u32)
        .unwrap_or(1);

    let elements = setup_elements(Rc::new(font), Rc::new(cute_font));

    let data = Data {
        width: 0,
        height: 0,
    };
    let mut state = Panel::new(
        elements,
        [0x00, 0x00, 0x00, 0xff],
        [0xff, 0xff, 0xff, 0xff],
        data,
    );

    let (width, height) = (state.width, state.height);
    // TODO: This is _SUCH_ a papercut or even pitfall, as I just saw.
    state.data_mut().width = width;
    state.data_mut().height = height;
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
                state.data_mut().width = ls.width;
                state.data_mut().height = ls.height;
                window.request_redraw();
            }
        }
    });
}

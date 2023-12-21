use block::{Block, DrawBlock};
use elements::Element;
use fleck::Font;
use pixels::Pixels;

mod block;
pub mod elements;

/// The number of bytes per [`Pixel`].
pub const PIXEL_SIZE: usize = 4;
pub type Pixel = [u8; PIXEL_SIZE];

/// An iterator over rows of [`Pixel`]s.
type Rows<'b> = std::slice::ChunksExact<'b, Pixel>;
/// An iterator over mutable rows of [`Pixel`]s.
type RowsMut<'b> = std::slice::ChunksExactMut<'b, Pixel>;

pub struct Raam<D> {
    pub width: u32,
    pub height: u32,
    pub foreground: Pixel,
    pub background: Pixel,

    font: Box<Font>,

    data: D,
    elements: Element<D>,
}

impl<D> Raam<D> {
    pub fn new(
        elements: Element<D>,
        font: Box<Font>,
        foreground: Pixel,
        background: Pixel,
        data: D,
    ) -> Self {
        Self {
            width: elements.block_width(&font) as u32,
            height: elements.block_height(&font) as u32,
            foreground,
            background,
            font,
            data,
            elements,
        }
    }

    pub fn data_mut(&mut self) -> &mut D {
        &mut self.data
    }

    /// Update all elements with the internal `data`.
    pub fn update(&mut self) {
        self.elements.update(&self.data)
    }

    pub fn draw(&self, pixels: &mut Pixels) {
        let mut block = Block::new(self.width as usize, self.height as usize, self.background);

        // Draw onto our block.
        block.paint(self.elements.block(&self.font), 0, 0);

        // Draw the block onto the pixels.
        block.draw_onto_pixels(pixels, 0);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

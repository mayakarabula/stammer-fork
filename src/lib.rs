use fleck::Font;
use pixels::Pixels;

use crate::elements::Element;

pub mod elements;

/// The number of bytes per [`Pixel`].
pub const PIXEL_SIZE: usize = 4;
pub type Pixel = [u8; PIXEL_SIZE];

/// An iterator over rows of [`Pixel`]s.
type Rows<'b> = std::slice::ChunksExact<'b, Pixel>;
/// An iterator over mutable rows of [`Pixel`]s.
type RowsMut<'b> = std::slice::ChunksExactMut<'b, Pixel>;

pub struct Raam {
    pub width: u32,
    pub height: u32,
    pub foreground: Pixel,
    pub background: Pixel,

    font: Box<Font>,

    elements: Element,
}

impl Raam {
    pub fn new(elements: Element, font: Box<Font>, foreground: Pixel, background: Pixel) -> Self {
        Self {
            width: elements.block_width(&font) as u32,
            height: elements.block_height(&font) as u32,
            foreground,
            background,
            font,
            elements,
        }
    }

    pub fn update(&mut self) {
        self.elements.update()
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

struct Block {
    /// Width in pixels.
    width: usize,
    /// Height in pixels.
    height: usize,
    /// Row-major pixel buffer.
    buf: Vec<Pixel>, // TODO: Invariant: buf.len() == width * height
}

impl Block {
    /// Creates a new [`Block`].
    fn new(width: usize, height: usize, background: Pixel) -> Self {
        Self {
            width,
            height,
            buf: vec![background; width * height],
        }
    }

    /// Returns an iterator over the rows in this [`Block`].
    ///
    /// # Panics
    ///
    /// If `self.width` is 0, the function will panic.
    fn rows(&self) -> Rows {
        self.buf.chunks_exact(self.width)
    }

    /// Returns an iterator over mutable rows in this [`Block`].
    ///
    /// # Panics
    ///
    /// If `self.width` is 0, the function will panic.
    fn rows_mut(&mut self) -> RowsMut {
        self.buf.chunks_exact_mut(self.width)
    }

    // TODO: Improve this weirdly worded doc comment.
    // TODO: Doc comment out of date because we now also do height.
    /// Paint another [`Block`] onto this one.
    ///
    /// If the remaining space in the `Block` after `start_x` is smaller than the width of the
    /// `other`, the first pixels of `other` are drawn up to the border of `self`.
    ///
    /// # Panics
    ///
    /// In case `start_x` is greater than the width of the `Block` that is painted onto
    /// (`self.width`), this function will panic.
    fn paint(&mut self, other: Self, start_x: usize, start_y: usize) {
        if other.width == 0 || self.width == 0 {
            return;
        }
        assert!(start_x <= self.width);
        assert!(start_y <= self.height);
        let delta_x = usize::min(other.width, self.width - start_x);
        let delta_y = usize::min(other.height, self.height - start_y);
        let end_x = start_x + delta_x;
        for (row, other_row) in self
            .rows_mut()
            .skip(start_y)
            .take(delta_y)
            .zip(other.rows())
        {
            row[start_x..end_x].copy_from_slice(&other_row[..delta_x])
        }
    }

    /// Draws this [`Block`]s contents onto the provided [`Pixels`].
    fn draw_onto_pixels(&self, pixels: &mut Pixels, start_x: usize) {
        // let size = pixels.texture().size();
        // assert_eq!(size.width as usize, self.width);
        // assert_eq!(size.height as usize, Self::HEIGHT);
        for (y, row) in self.rows().enumerate() {
            let idx = (y * self.width + start_x) * PIXEL_SIZE;
            // TODO: See if we can get rid of this iter(). Perhaps through feature(slice_flatten)?
            // TODO: Where should the .copied() go, ideally?
            let row_bytes: Vec<_> = row.iter().copied().flatten().collect();
            pixels.frame_mut()[idx..idx + row_bytes.len()].copy_from_slice(&row_bytes);
        }
    }
}

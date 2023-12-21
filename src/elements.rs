use std::collections::VecDeque;

use fleck::Font;

use crate::block::DrawBlock;
use crate::wrapped_text::WrappedText;
use crate::{Block, PIXEL_SIZE};

type UpdateFn<D> = fn(&mut ElementKind<D>, &D);

pub struct Element<D> {
    /// Function that will update `thing` according to some `data` with type `D`.
    inner_update: Option<UpdateFn<D>>, // TODO: Bikeshedded name.
    thing: ElementKind<D>,
}

impl<D> Element<D> {
    pub fn new(update: Option<UpdateFn<D>>, thing: ElementKind<D>) -> Self {
        Self {
            inner_update: update,
            thing,
        }
    }

    pub fn still(thing: ElementKind<D>) -> Self {
        Self::new(None, thing)
    }

    pub fn dynamic(update: UpdateFn<D>, thing: ElementKind<D>) -> Self {
        Self::new(Some(update), thing)
    }

    /// Update the inner [`ElementKind`] and subsequently let these children call their own update
    /// functions as well, in the case of a collection [`ElementKind`], such as
    /// [`ElementKind::Row`] or [`ElementKind::Stack`].
    pub(crate) fn update(&mut self, data: &D) {
        if let Some(update) = self.inner_update {
            update(&mut self.thing, data)
        }

        // TODO: (easy) Break out the 'is it a collection type' logic to a
        // `ElementKind::is_collection(&self) -> bool`.
        match &mut self.thing {
            // Update the chirren.
            ElementKind::Row(elements) | ElementKind::Stack(elements) => {
                elements.iter_mut().for_each(|element| element.update(data))
            }
            // These are not collection ElementKinds, and therefore do not require their children
            // to be updated.
            ElementKind::Space
            | ElementKind::Text(_)
            | ElementKind::Paragraph(_, _, _)
            | ElementKind::Graph(_) => {}
        };
    }
}

impl<D> DrawBlock for Element<D> {
    fn block_width(&self, font: &Font) -> usize {
        self.thing.block_width(font)
    }

    fn block_height(&self, _font: &Font) -> usize {
        self.thing.block_height(_font)
    }

    fn block(&self, font: &Font) -> Block {
        self.thing.block(font)
    }
}

pub enum ElementKind<D> {
    Space,

    Text(String),
    Paragraph(WrappedText, usize, usize),
    Graph(Graph),

    Row(Vec<Element<D>>),
    Stack(Vec<Element<D>>),
}

impl<D> DrawBlock for ElementKind<D> {
    // TODO: See, this with the font is where my design starts breaking down. I think the a
    // reference to the font should be part of the element.
    fn block_width(&self, font: &Font) -> usize {
        match self {
            ElementKind::Space => font.max_width(),
            ElementKind::Text(t) => font.determine_width(t),
            ElementKind::Paragraph(_, width, _) => *width,
            ElementKind::Graph(g) => g.len(),
            ElementKind::Row(row) => row
                .iter()
                .map(|element| element.thing.block_width(font))
                .sum(),
            ElementKind::Stack(stack) => stack
                .iter()
                .map(|element| element.thing.block_width(font))
                .max()
                .unwrap_or_default(),
        }
    }

    // TODO: Consider not relying on font here. May actually be more correct, since we always have
    // the same Font::GLYPH_HEIGHT?
    fn block_height(&self, _font: &Font) -> usize {
        match self {
            ElementKind::Space | ElementKind::Text(_) | ElementKind::Graph(_) => Font::GLYPH_HEIGHT,
            ElementKind::Paragraph(_, _, height) => *height,
            ElementKind::Row(row) => row
                .iter()
                .map(|element| element.thing.block_height(_font))
                .max()
                .unwrap_or_default(),
            ElementKind::Stack(stack) => stack
                .iter()
                .map(|element| element.thing.block_height(_font))
                .sum(),
        }
    }

    fn block(&self, font: &Font) -> Block {
        let background = [0xff; PIXEL_SIZE];
        let foreground = [0x77, 0x33, 0x22, 0xff];

        let mut block = Block::new(self.block_width(font), self.block_height(font), background);

        match self {
            ElementKind::Space => {}
            ElementKind::Text(text) => {
                let glyphs = text.chars().flat_map(|ch| font.glyph(ch));
                let width: usize = glyphs.clone().map(|glyph| glyph.width as usize).sum();
                let mut x0 = 0;
                for glyph in glyphs {
                    let glyph_width = glyph.width as usize;
                    for (y, row) in glyph.enumerate() {
                        for (xg, cell) in row.enumerate() {
                            let x = x0 + xg;
                            // TODO: This may be more efficient than what I did in Graph. May be
                            // worth investigating which is better.
                            block.buf[y * width + x] = if cell { foreground } else { background };
                        }
                    }
                    x0 += glyph_width;
                }
            }
            ElementKind::Paragraph(text, _, height) => {
                let mut y = 0;
                for line in text
                    .as_ref()
                    .lines()
                    .map(|line| ElementKind::<D>::Text(line.to_string()))
                {
                    let line_block = line.block(font);
                    let line_block_height = line_block.height;
                    block.paint(line_block, 0, y);
                    y += line_block_height;
                    if y > *height {
                        break;
                    }
                }
            }
            ElementKind::Graph(graph) => {
                let min = graph.min();
                let max = graph.max();
                let height = block.height - 1;
                let points = graph
                    .iter()
                    .map(|v| height - ((v - min) / (max - min) * height as f32).round() as usize);
                let mut rows: Vec<_> = block.rows_mut().collect();
                for (x, y) in points.enumerate() {
                    rows[y][x] = [0x66, 0x33, 0x99, 0xff]
                }
            }
            ElementKind::Row(row) => {
                let row_blocks = row.iter().map(|element| element.thing.block(font));
                let mut x = 0;
                for row_block in row_blocks {
                    let width = row_block.width;
                    block.paint(row_block, x, 0);
                    x += width;
                }
            }
            ElementKind::Stack(stack) => {
                let stack_blocks = stack.iter().map(|element| element.thing.block(font));
                let mut y = 0;
                for stack_block in stack_blocks {
                    let height = stack_block.height;
                    block.paint(stack_block, 0, y);
                    y += height;
                }
            }
        }

        block
    }
}

pub struct Graph(VecDeque<f32>);

impl Graph {
    pub fn new(size: usize) -> Self {
        let mut inner = VecDeque::new();
        inner.resize(size, 0.0);
        Self(inner)
    }

    pub fn push(&mut self, value: f32) {
        let Self(inner) = self;
        let size = inner.len();
        inner.truncate(size.saturating_sub(1)); // Truncate to shift values along.
        inner.push_front(value)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &f32> {
        self.0.iter()
    }

    pub fn inner_mut(&mut self) -> &mut VecDeque<f32> {
        &mut self.0
    }

    pub fn min(&self) -> f32 {
        if self.is_empty() {
            return Default::default();
        }
        // TODO: Check out the perf on this .copied()
        self.iter().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn max(&self) -> f32 {
        if self.is_empty() {
            return Default::default();
        }
        self.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }
}

impl From<VecDeque<f32>> for Graph {
    fn from(deque: VecDeque<f32>) -> Self {
        Self(deque)
    }
}

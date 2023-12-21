use std::collections::VecDeque;

use fleck::Font;

use crate::wrapped_text::WrappedText;
use crate::{Block, PIXEL_SIZE};

pub enum Element {
    Space,

    Text(String),
    Paragraph(WrappedText, usize, usize),
    Graph(Graph),

    Row(Vec<Element>),
    Stack(Vec<Element>),
}

impl Element {
    pub(crate) fn update(&mut self) {
        match self {
            Element::Graph(graph) => graph.0.rotate_left(1),
            Element::Row(elements) | Element::Stack(elements) => {
                for element in elements {
                    element.update()
                }
            }
            Element::Space | Element::Text(_) | Element::Paragraph(_, _, _) => {}
        }
    }

    // TODO: See, this with the font is where my design starts breaking down. I think the a
    // reference to the font should be part of the element.
    pub(crate) fn block_width(&self, font: &Font) -> usize {
        match self {
            Element::Space => font.max_width(),
            Element::Text(t) => font.determine_width(t),
            Element::Paragraph(_, width, _) => *width,
            Element::Graph(g) => g.len(),
            Element::Row(row) => row.iter().map(|element| element.block_width(font)).sum(),
            Element::Stack(stack) => stack
                .iter()
                .map(|element| element.block_width(font))
                .max()
                .unwrap_or_default(),
        }
    }

    // TODO: Consider not relying on font here. May actually be more correct, since we always have
    // the same Font::GLYPH_HEIGHT?
    pub(crate) fn block_height(&self, _font: &Font) -> usize {
        match self {
            Element::Space | Element::Text(_) | Element::Graph(_) => Font::GLYPH_HEIGHT,
            Element::Paragraph(_, _, height) => *height,
            Element::Row(row) => row
                .iter()
                .map(|element| element.block_height(_font))
                .max()
                .unwrap_or_default(),
            Element::Stack(stack) => stack
                .iter()
                .map(|element| element.block_height(_font))
                .sum(),
        }
    }

    pub(crate) fn block(&self, font: &Font) -> Block {
        let background = [0xff; PIXEL_SIZE];
        let foreground = [0x77, 0x33, 0x22, 0xff];

        let mut block = Block::new(self.block_width(font), self.block_height(font), background);

        match self {
            Element::Space => {}
            Element::Text(text) => {
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
            Element::Paragraph(text, _, height) => {
                let mut y = 0;
                for line in text
                    .as_ref()
                    .lines()
                    .map(|line| Element::Text(line.to_string()))
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
            Element::Graph(graph) => {
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
            Element::Row(row) => {
                let row_blocks = row.iter().map(|element| element.block(font));
                let mut x = 0;
                for row_block in row_blocks {
                    let width = row_block.width;
                    block.paint(row_block, x, 0);
                    x += width;
                }
            }
            Element::Stack(stack) => {
                let stack_blocks = stack.iter().map(|element| element.block(font));
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

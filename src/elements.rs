use std::collections::VecDeque;

use fleck::Font;

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

pub struct WrappedText(String);

impl WrappedText {
    /// Creates a new [`WrappedText`] that will be wrapped to the specified `width` and according
    /// to the glyphs in the provided [`Font`].
    pub fn new(text: &str, width: usize, font: &Font) -> Self {
        // TODO: I don't know whether this makes any sense. Never measured it. I like it because it
        // may prevent two allocations but also, who cares.
        if text
            .lines()
            .map(|line| font.determine_width(line))
            .all(|lw| lw <= width)
        {
            return Self(text.to_string());
        }

        // Please note that this is not a particularly good implementation.
        let mut wrapped = String::new();
        let mut scrap = String::new(); // Space to build a new line before pushing it to wrapped.
        for line in text.lines() {
            // If it already would fit well, we don't need to do anything.
            if font.determine_width(line) <= width {
                wrapped.push_str(line);
                continue;
            }

            // Otherwise, we will have to wrap the line ourselves.
            let mut line_width = 0; // The pixel width of the line under construction (`scrap`).
            for ch in line.chars() {
                let ch_width = font
                    .glyph(ch)
                    .map(|gl| gl.width as usize)
                    .unwrap_or_default();
                if line_width + ch_width > width {
                    if let Some(breakpoint) = scrap.rfind(char::is_whitespace) {
                        wrapped.push_str(&scrap[..breakpoint]);
                        wrapped.push('\n');
                        // The `overhang` is the part of `scrap` after the breaking whitespace.
                        let overhang = &scrap[breakpoint + 1..];
                        line_width = font.determine_width(overhang);
                        // We want to internally copy the overhang onto the start of the string.
                        // An equivalent method would be `scrap = overhang.to_string()`, but by
                        // doing it like this, we avoid an allocation.
                        let scrap_head = scrap.as_ptr() as *mut u8;
                        for (i, &b) in overhang.as_bytes().iter().enumerate() {
                            // Safety: The length of `overhang` is equal or greater than the length
                            // of `scrap`, since `overhang` is a slice of `scrap`. Therefore, the
                            // pointer add will always be in bounds.
                            unsafe { std::ptr::write(scrap_head.wrapping_add(i), b) }
                        }
                        scrap.truncate(overhang.len());
                    } else {
                        wrapped.push_str(&scrap);
                        wrapped.push('\n');
                        scrap.clear();
                        line_width = 0;
                    }
                }

                scrap.push(ch);
                line_width += ch_width;
            }

            scrap.clear();
            wrapped.push('\n');
        }

        WrappedText(wrapped)
    }

    /// Get the inner wrapped [`String`], consuming the [`WrappedText`].
    pub fn unveil(self) -> String {
        self.0
    }
}

impl AsRef<str> for WrappedText {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl ToString for WrappedText {
    /// # Note
    ///
    /// To get the inner [`String`] directly, use [`WrappedText::unveil`].
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

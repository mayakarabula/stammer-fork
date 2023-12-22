use fleck::Font;
pub use graph::Graph;
pub use wrapped_text::WrappedText;

use crate::block::DrawBlock;
use crate::{Block, PIXEL_SIZE};

pub mod graph;
pub mod wrapped_text;

type UpdateFn<D> = fn(&mut ElementKind<D>, &D);

pub struct Element<D> {
    /// If `None`, the width is determined based on the [`Element`] content. If set with
    /// `Some(width)`, this width is taken to be this `Element`s width.
    width: Option<usize>,
    /// Function that will update `thing` according to some `data` with type `D`.
    inner_update: Option<UpdateFn<D>>, // TODO: Bikeshedded name.
    thing: ElementKind<D>,
}

impl<D> Element<D> {
    pub fn new(update: Option<UpdateFn<D>>, thing: ElementKind<D>) -> Self {
        Self {
            width: None,
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

    /// Explicitly set a fixed width for this [`Element`].
    pub fn with_fixed_width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Let the width of this [`Element`] adapt based on its content.
    pub fn with_auto_width(mut self) -> Self {
        self.width = None;
        self
    }

    /// Set the width of this [`Element`] according to its content at the moment this function is
    /// called.
    pub fn with_baked_width(mut self, font: &Font) -> Self {
        self.width = Some(self.block_width(font));
        self
    }
}

impl<D> Element<D> {
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
            | ElementKind::Padding(_)
            | ElementKind::Text(_)
            | ElementKind::Paragraph(_, _, _)
            | ElementKind::Graph(_) => {}
        };
    }
}

impl<D> DrawBlock for Element<D> {
    // TODO: See, this with the font is where my design starts breaking down. I think the a
    // reference to the font should be part of the element.
    fn block_width(&self, font: &Font) -> usize {
        // If the width is set explicitly on the [`Element`], that has priority.
        if let Some(width) = self.width {
            return width;
        }

        // Otherwise, calculate the width from the current content.
        match &self.thing {
            ElementKind::Space => font.determine_width("  ") as usize,
            ElementKind::Padding(width) => *width,
            ElementKind::Text(t) => font.determine_width(&t),
            ElementKind::Paragraph(_, width, _) => *width,
            ElementKind::Graph(g) => g.len(),
            ElementKind::Row(row) => row.iter().map(|element| element.block_width(font)).sum(),
            ElementKind::Stack(stack) => stack
                .iter()
                .map(|element| element.block_width(font))
                .max()
                .unwrap_or_default(),
        }
    }

    // TODO: Consider not relying on font here. May actually be more correct, since we always have
    // the same Font::GLYPH_HEIGHT?
    fn block_height(&self, _font: &Font) -> usize {
        match &self.thing {
            ElementKind::Space
            | ElementKind::Padding(_)
            | ElementKind::Text(_)
            | ElementKind::Graph(_) => Font::GLYPH_HEIGHT,
            ElementKind::Paragraph(_, _, height) => *height,
            ElementKind::Row(row) => row
                .iter()
                .map(|element| element.block_height(_font))
                .max()
                .unwrap_or_default(),
            ElementKind::Stack(stack) => stack
                .iter()
                .map(|element| element.block_height(_font))
                .sum(),
        }
    }

    fn block(&self, font: &Font) -> Block {
        // TODO: Let this point to the Element's style, once we have that.
        let background = [0xff; PIXEL_SIZE];
        let foreground = [0x77, 0x33, 0x22, 0xff];

        let width = self.block_width(font);
        let mut block = Block::new(width, self.block_height(font), background);

        fn draw_text(
            block: &mut Block,
            text: &str,
            width: usize,
            font: &Font,
            foreground: [u8; 4],
            background: [u8; 4],
        ) {
            let glyphs = text.chars().flat_map(|ch| font.glyph(ch));
            let mut x0 = 0;
            for glyph in glyphs {
                let glyph_width = glyph.width as usize;
                if x0 >= width {
                    break;
                }
                for (y, row) in glyph.enumerate() {
                    for (xg, cell) in row.enumerate() {
                        let x = x0 + xg;
                        if x >= width {
                            continue;
                        }
                        // TODO: This may be more efficient than what I did in Graph. May be
                        // worth investigating which is better.
                        block.buf[y * width + x] = if cell { foreground } else { background };
                    }
                }
                x0 += glyph_width;
            }
        }

        match &self.thing {
            ElementKind::Space | ElementKind::Padding(_) => {}
            ElementKind::Text(text) => {
                draw_text(&mut block, text, width, font, foreground, background)
            }
            ElementKind::Paragraph(text, _, height) => {
                let mut y = 0;
                for line in text.as_ref().lines().map(|line| line.to_string()) {
                    let width = font.determine_width(&line);
                    let mut line_block = Block::new(width, *height, background);
                    draw_text(&mut line_block, &line, width, font, foreground, background);
                    let line_block_height = line_block.height;
                    block.paint(line_block, 0, y);
                    y += line_block_height; // FIXME
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
                let row_blocks = row.iter().map(|element| element.block(font));
                let mut x = 0;
                for row_block in row_blocks {
                    let width = row_block.width;
                    block.paint(row_block, x, 0);
                    x += width;
                }
            }
            ElementKind::Stack(stack) => {
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

pub enum ElementKind<D> {
    Space,
    Padding(usize),

    Text(String),
    Paragraph(WrappedText, usize, usize),
    Graph(Graph),

    Row(Vec<Element<D>>),
    Stack(Vec<Element<D>>),
}

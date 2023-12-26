use std::rc::Rc;

use fleck::Font;
pub use graph::Graph;
pub use wrapped_text::WrappedText;

use crate::block::DrawBlock;
use crate::{Block, Pixel};

pub mod graph;
pub mod wrapped_text;

type UpdateFn<D> = fn(&mut Element<D>, &D);

#[derive(Debug, Default, Clone, Copy)]
/// # Note
///
/// The [`width`] and [`height`] fields are the unconstrained dimensions. These may fall outside of
/// the bounds set by the `min` and `max` fields. Use [`Element::fill_size`] and
/// [`Element::overall_size`] to get the actual sizes constrained by the `min` and `max`
/// properties.
pub struct Size {
    width: u32,
    height: u32,
    pub maxwidth: Option<u32>,
    pub maxheight: Option<u32>,
    pub minwidth: Option<u32>,
    pub minheight: Option<u32>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Padding {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

#[derive(Debug, Clone)]
pub struct Style {
    pub foreground: Pixel,
    pub background: Pixel,
    pub font: Rc<Font>,
}

impl Style {
    pub fn new(foreground: Pixel, background: Pixel, font: Rc<Font>) -> Self {
        Self {
            foreground,
            background,
            font,
        }
    }

    /// Black foreground, white background.
    pub fn default_with_font(font: Rc<Font>) -> Self {
        Self {
            foreground: [0x00, 0x00, 0x00, 0xff],
            background: [0xff, 0xff, 0xff, 0xff],
            font,
        }
    }
}

pub struct Element<D> {
    pub size: Size,
    pub padding: Padding,
    pub style: Style,
    update: Option<UpdateFn<D>>,
    pub content: Content<D>,
}

pub enum Content<D> {
    Text(String),
    Paragraph(WrappedText),
    Row(Vec<Element<D>>),
    Stack(Vec<Element<D>>),
}

impl<D> Element<D> {
    pub fn new(update: Option<UpdateFn<D>>, content: Content<D>, font: Rc<Font>) -> Self {
        Self {
            size: Default::default(),
            padding: Default::default(),
            style: Style::default_with_font(font),
            update,
            content,
        }
    }

    pub fn still(font: Rc<Font>, content: Content<D>) -> Self {
        Self::new(None, content, font)
    }

    pub fn dynamic(update: UpdateFn<D>, font: Rc<Font>, content: Content<D>) -> Self {
        Self::new(Some(update), content, font)
    }

    /* min size */
    pub fn with_minwidth(mut self, minwidth: u32) -> Self {
        self.size.minwidth = Some(minwidth);
        self
    }

    pub fn with_minheight(mut self, minheight: u32) -> Self {
        self.size.minheight = Some(minheight);
        self
    }

    /* max size */
    pub fn with_maxwidth(mut self, maxwidth: u32) -> Self {
        self.size.maxwidth = Some(maxwidth);
        self
    }

    pub fn with_maxheight(mut self, maxheight: u32) -> Self {
        self.size.maxheight = Some(maxheight);
        self
    }

    /* fixed size */
    pub fn with_fixedwidth(mut self, width: u32) -> Self {
        self.size.minwidth = Some(width);
        self.size.maxwidth = Some(width);
        self
    }

    pub fn with_fixedheight(mut self, height: u32) -> Self {
        self.size.minheight = Some(height);
        self.size.maxheight = Some(height);
        self
    }

    /* padding */
    pub fn with_padding_top(mut self, padding: u32) -> Self {
        self.padding.top = padding;
        self
    }

    pub fn with_padding_bottom(mut self, padding: u32) -> Self {
        self.padding.bottom = padding;
        self
    }

    pub fn with_padding_left(mut self, padding: u32) -> Self {
        self.padding.left = padding;
        self
    }

    pub fn with_padding_right(mut self, padding: u32) -> Self {
        self.padding.right = padding;
        self
    }

    /* style */
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_foreground(mut self, foreground: Pixel) -> Self {
        self.style.foreground = foreground;
        self
    }

    pub fn with_background(mut self, background: Pixel) -> Self {
        self.style.background = background;
        self
    }
}

impl<D> Element<D> {
    /// Configure the internal size information based on the [`Element`]'s content and constraints.
    ///
    /// For container [`Content`]s, this method is called recursively.
    ///
    /// After calling this method, the `Element`'s width and height as accessed through
    /// [`Element::fill_size`] or [`Element::overall_size`] are reliably defined and are valid
    /// until the content of it or any of its children are mutated.
    ///
    /// In case of an `Element` with content [`Content::Paragraph`], the inner text is wrapped to
    /// the `maxwidth`, and the `width` and `height` are subsequently calculated based on these
    /// wrapped lines.
    pub(crate) fn bake_size(&mut self) {
        match &mut self.content {
            Content::Text(text) => {
                self.size.width = self.style.font.determine_width(text) as u32;
                self.size.height = self.style.font.height() as u32;
            }
            Content::Paragraph(wrapped) => {
                wrapped.rewrap(self.size.maxwidth, &self.style.font);
                self.size.width = wrapped
                    .lines()
                    .map(|line| self.style.font.determine_width(line) as u32)
                    .max()
                    .unwrap_or_default();
                self.size.height = (self.style.font.height() * wrapped.lines_count()) as u32;
            }
            Content::Row(children) | Content::Stack(children) => {
                // TODO: See whether this collect alloc can be eliminated. Perhaps unzip?
                let sizes: Vec<_> = children
                    .iter_mut()
                    .map(|child| {
                        child.bake_size();
                        child.overall_size()
                    })
                    .collect();
                let widths = sizes.iter().map(|size| size.width);
                let heights = sizes.iter().map(|size| size.height);
                match self.content {
                    Content::Row(_) => {
                        self.size.width = widths.sum();
                        self.size.height = heights.max().unwrap_or_default();
                    }
                    Content::Stack(_) => {
                        self.size.height = heights.sum();
                        self.size.width = widths.max().unwrap_or_default();
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    pub(crate) fn update(&mut self, data: &D) {
        if let Some(update) = self.update {
            update(self, data)
        }

        // TODO: (easy) Break out the 'is it a collection type' logic to a
        // `Content::is_collection(&self) -> bool`.
        match &mut self.content {
            // Update the chirren.
            Content::Row(elements) | Content::Stack(elements) => {
                elements.iter_mut().for_each(|element| element.update(data))
            }
            _ => {}
        };
    }
}

impl<D> Element<D> {
    pub fn size(&self) -> Size {
        self.size
    }

    pub fn fill_size(&self) -> Dimensions {
        let size = self.size();
        let width = match (size.minwidth, size.maxwidth) {
            (None, None) => size.width,
            (None, Some(maxwidth)) => size.width.min(maxwidth),
            (Some(minwidth), None) => size.width.max(minwidth),
            (Some(minwidth), Some(maxwidth)) => size.width.clamp(minwidth, maxwidth),
        };
        let height = match (size.minheight, size.maxheight) {
            (None, None) => size.height,
            (None, Some(maxheight)) => size.height.min(maxheight),
            (Some(minheight), None) => size.height.max(minheight),
            (Some(minheight), Some(maxheight)) => size.height.clamp(minheight, maxheight),
        };
        Dimensions { width, height }
    }

    /* without padding */
    pub fn min_fill_size(&self) -> Dimensions {
        let size = self.size();
        let width = match (size.minwidth, size.maxwidth) {
            (None, None) => size.width,
            (None, Some(maxwidth)) => size.width.min(maxwidth),
            (Some(minwidth), _) => minwidth,
        };
        let height = match (size.minheight, size.maxheight) {
            (None, None) => size.height,
            (None, Some(maxheight)) => size.height.min(maxheight),
            (Some(minheight), _) => minheight,
        };
        Dimensions { width, height }
    }

    pub fn max_fill_size(&self) -> Dimensions {
        let size = self.size();
        let width = match (size.minwidth, size.maxwidth) {
            (None, None) => size.width,
            (Some(minwidth), None) => size.width.max(minwidth),
            (_, Some(maxwidth)) => maxwidth,
        };
        let height = match (size.minheight, size.maxheight) {
            (None, None) => size.height,
            (Some(minheight), None) => size.height.max(minheight),
            (_, Some(maxheight)) => maxheight,
        };
        Dimensions { width, height }
    }

    /* with padding */
    fn include_padding(&self, mut fillsize: Dimensions) -> Dimensions {
        fillsize.width += self.padding.left + self.padding.right;
        fillsize.height += self.padding.top + self.padding.bottom;
        fillsize
    }

    pub fn overall_size(&self) -> Dimensions {
        self.include_padding(self.fill_size())
    }

    pub fn min_overall_size(&self) -> Dimensions {
        self.include_padding(self.min_fill_size())
    }

    pub fn max_overall_size(&self) -> Dimensions {
        self.include_padding(self.max_fill_size())
    }
}

impl<D> DrawBlock for Element<D> {
    fn block(&self) -> Block {
        let Dimensions { width, height } = self.fill_size();
        let mut block = Block::new(width as usize, height as usize, self.style.background);

        match &self.content {
            Content::Text(text) => draw_text(
                &mut block,
                text,
                Alignment::default(), // TODO: Give Text an alignment field.
                &self.style.font,
                self.style.foreground,
                self.style.background,
            ),
            Content::Paragraph(wrapped) => {
                let mut y = 0;
                for line in wrapped.lines() {
                    let mut line_block = Block::new(
                        width as usize,
                        self.style.font.height(),
                        self.style.background,
                    );
                    draw_text(
                        &mut line_block,
                        line,
                        Alignment::default(), // TODO: Give Paragraph an alignment field.
                        &self.style.font,
                        self.style.foreground,
                        self.style.background,
                    );
                    let line_block_height = line_block.height;
                    block.paint(line_block, 0, y);
                    y += line_block_height; // FIXME
                    if y > height as usize {
                        break;
                    }
                }
            }
            Content::Row(row) => {
                let mut x = 0;
                for element in row {
                    x += element.padding.left;
                    let row_block = element.block();
                    debug_assert_eq!(element.fill_size().width as usize, row_block.width);
                    block.paint(row_block, x as usize, 0);
                    x += element.fill_size().width + element.padding.right;
                }
            }
            Content::Stack(stack) => {
                let mut y = 0;
                for element in stack {
                    y += element.padding.top;
                    let stack_block = element.block();
                    debug_assert_eq!(element.fill_size().height as usize, stack_block.height);
                    block.paint(stack_block, 0, y as usize);
                    y += element.fill_size().height + element.padding.bottom;
                }
            }
        }

        block
    }
}

#[derive(Default, Clone, Copy)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

#[inline(always)]
fn draw_text(
    block: &mut Block,
    text: &str,
    alignment: Alignment,
    font: &Font,
    foreground: Pixel,
    background: Pixel,
) {
    // TODO: This implementation is not ideal. Since a Block is created to fit the entire
    // line of text to then be cut off in a manner that depends on the alignment, we do
    // more allocations than is necessary, and we may render more characters into that
    // Block than is necessary (since they are eventually thrown away due to alignment).
    // A better way of implementing this would be to do the proper computer maths with
    // indices, so that it can all be done through iterators (which evaluate lazily).
    // As a note for such an implementation in the future, see the state at or around
    // commit e945006.

    let scrap_width = font.determine_width(text);
    // TODO: Perhaps this case can be handled a little more gracefull, but that will
    // require a more holistic view of the whole layout "engine" in a later stage.
    // Postponing ;)
    if block.width == 0 || scrap_width == 0 {
        return; // Nothing to even draw, here. Why expend the energy?
    }
    let mut scrap = Block::new(scrap_width, block.height, background);
    let glyphs = text.chars().flat_map(|ch| font.glyph(ch));
    let mut x0 = 0;
    for glyph in glyphs {
        let glyph_width = glyph.width as usize;
        for (y, row) in glyph.enumerate() {
            for (xg, cell) in row.enumerate() {
                let x = x0 + xg;
                // TODO: This may be more efficient than what I did in Graph. May be
                // worth investigating which is better.
                scrap.buf[y * scrap.width + x] = if cell { foreground } else { background };
            }
        }
        x0 += glyph_width;
    }

    // TODO: Not loving how this match turned out. Seems kind of messy and with repetition
    // that is confusing, since it obscures a more elegant insight.
    match alignment {
        Alignment::Left => {
            let end = usize::min(block.width, scrap.width);
            block
                .rows_mut()
                .zip(scrap.rows())
                .for_each(|(row, scrap_row)| {
                    row[..end].copy_from_slice(&scrap_row[..end]);
                })
        }
        // TODO: The first two branches do the exact same thing. Figure out how to do this
        // nicely.
        // TODO: Decide on the actually correct behavior for Alignment::Center in this case.
        Alignment::Center if scrap.width >= block.width => {
            let end = usize::min(block.width, scrap.width);
            block
                .rows_mut()
                .zip(scrap.rows())
                .for_each(|(row, scrap_row)| {
                    row[..end].copy_from_slice(&scrap_row[..end]);
                })
        }
        Alignment::Center => {
            let remainder = block.width - scrap.width;
            let start = remainder / 2;
            block
                .rows_mut()
                .zip(scrap.rows())
                .for_each(|(row, scrap_row)| {
                    row[start..start + scrap.width].copy_from_slice(scrap_row);
                })
        }
        Alignment::Right => {
            let rstart = block.width.saturating_sub(scrap.width);
            let sstart = scrap.width.saturating_sub(block.width);
            block
                .rows_mut()
                .zip(scrap.rows())
                .for_each(|(row, scrap_row)| {
                    row[rstart..].copy_from_slice(&scrap_row[sstart..]);
                })
        }
    }
}

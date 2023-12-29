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
pub enum SizingStrategy {
    #[default]
    /// Just do whatever. The "normal" behavior.
    Whatever,
    /// Hey, please use as little room as possible please.
    Chonker,
    /// Pretty please fill out the room as much as you can!
    Smollest,
}

#[derive(Debug, Default, Clone, Copy)]
/// # Note
///
/// The [`width`] and [`height`] fields are the unconstrained dimensions. These may fall outside of
/// the bounds set by the `min` and `max` fields. Use [`Element::fill_size`] and
/// [`Element::overall_size`] to get the actual sizes constrained by the `min` and `max`
/// properties.
pub struct Size {
    pub strategy: SizingStrategy,
    baked_width: u32,
    baked_height: u32,
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

type Pad = u32;

#[derive(Debug, Default, Clone, Copy)]
pub struct Padding {
    pub top: Pad,
    pub bottom: Pad,
    pub left: Pad,
    pub right: Pad,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Flex {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl Flex {
    fn vertical_flexes(&self) -> usize {
        self.top as usize + self.bottom as usize
    }

    fn horizontal_flexes(&self) -> usize {
        self.left as usize + self.right as usize
    }
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
    pub flex: Flex,
    pub style: Style,
    update: Option<UpdateFn<D>>,
    pub content: Content<D>,
}

pub enum Content<D> {
    Text(String, Alignment),
    Paragraph(WrappedText, Alignment),
    Row(Vec<Element<D>>),
    Stack(Vec<Element<D>>),
}

impl<D> Element<D> {
    pub fn new(update: Option<UpdateFn<D>>, content: Content<D>, font: Rc<Font>) -> Self {
        Self {
            size: Default::default(),
            padding: Default::default(),
            flex: Default::default(),
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

    /* strategy */
    pub fn with_strategy(mut self, strategy: SizingStrategy) -> Self {
        self.size.strategy = strategy;
        self
    }

    /* padding */
    pub fn with_padding_top(mut self, padding: Pad) -> Self {
        self.padding.top = padding;
        self
    }

    pub fn with_padding_bottom(mut self, padding: Pad) -> Self {
        self.padding.bottom = padding;
        self
    }

    pub fn with_padding_left(mut self, padding: Pad) -> Self {
        self.padding.left = padding;
        self
    }

    pub fn with_padding_right(mut self, padding: Pad) -> Self {
        self.padding.right = padding;
        self
    }

    /* flex */
    pub fn with_flex_top(mut self, flex: bool) -> Self {
        self.flex.top = flex;
        self
    }

    pub fn with_flex_bottom(mut self, flex: bool) -> Self {
        self.flex.bottom = flex;
        self
    }

    pub fn with_flex_left(mut self, flex: bool) -> Self {
        self.flex.left = flex;
        self
    }

    pub fn with_flex_right(mut self, flex: bool) -> Self {
        self.flex.right = flex;
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
        let width;
        let height;
        match &mut self.content {
            Content::Text(text, _) => {
                width = self.style.font.determine_width(text) as u32;
                height = self.style.font.height() as u32;
            }
            Content::Paragraph(wrapped, _) => {
                wrapped.rewrap(self.size.maxwidth, &self.style.font);
                width = wrapped
                    .lines()
                    .map(|line| self.style.font.determine_width(line) as u32)
                    .max()
                    .unwrap_or_default();
                height = (self.style.font.height() * wrapped.lines_count()) as u32;
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
                        width = widths.sum();
                        height = heights.max().unwrap_or_default();
                    }
                    Content::Stack(_) => {
                        height = heights.sum();
                        width = widths.max().unwrap_or_default();
                    }
                    _ => unreachable!(),
                }
            }
        }

        match self.size.strategy {
            SizingStrategy::Whatever => {
                self.size.baked_width = width;
                self.size.baked_height = height;
            }
            SizingStrategy::Chonker => {
                self.size.baked_width = self.size.maxwidth.unwrap_or_default().max(width);
                self.size.baked_height = self.size.maxheight.unwrap_or_default().max(height);
            }
            SizingStrategy::Smollest => {
                // Ah okay wow just realized what the problem is while I was doing something else.
                // Wait no I was wrong.
                self.size.baked_width = self.size.minwidth.unwrap_or(width).min(width);
                self.size.baked_height = self.size.minheight.unwrap_or(height).min(height);
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
            (None, None) => size.baked_width,
            (None, Some(maxwidth)) => size.baked_width.min(maxwidth),
            (Some(minwidth), None) => size.baked_width.max(minwidth),
            (Some(minwidth), Some(maxwidth)) => size.baked_width.clamp(minwidth, maxwidth),
        };
        let height = match (size.minheight, size.maxheight) {
            (None, None) => size.baked_height,
            (None, Some(maxheight)) => size.baked_height.min(maxheight),
            (Some(minheight), None) => size.baked_height.max(minheight),
            (Some(minheight), Some(maxheight)) => size.baked_height.clamp(minheight, maxheight),
        };
        Dimensions { width, height }
    }

    /* without padding */
    pub fn min_fill_size(&self) -> Dimensions {
        let size = self.size();
        let width = match (size.minwidth, size.maxwidth) {
            (None, None) => size.baked_width,
            (None, Some(maxwidth)) => size.baked_width.min(maxwidth),
            (Some(minwidth), _) => minwidth,
        };
        let height = match (size.minheight, size.maxheight) {
            (None, None) => size.baked_height,
            (None, Some(maxheight)) => size.baked_height.min(maxheight),
            (Some(minheight), _) => minheight,
        };
        Dimensions { width, height }
    }

    pub fn max_fill_size(&self) -> Dimensions {
        let size = self.size();
        let width = match (size.minwidth, size.maxwidth) {
            (None, None) => size.baked_width,
            (Some(minwidth), None) => size.baked_width.max(minwidth),
            (_, Some(maxwidth)) => maxwidth,
        };
        let height = match (size.minheight, size.maxheight) {
            (None, None) => size.baked_height,
            (Some(minheight), None) => size.baked_height.max(minheight),
            (_, Some(maxheight)) => maxheight,
        };
        Dimensions { width, height }
    }

    /* with padding */
    fn include_padding(mut fillsize: Dimensions, padding: Padding) -> Dimensions {
        fillsize.width += padding.left + padding.right;
        fillsize.height += padding.top + padding.bottom;
        fillsize
    }

    pub fn overall_size(&self) -> Dimensions {
        Self::include_padding(self.fill_size(), self.padding)
    }
}

impl<D> DrawBlock for Element<D> {
    fn block(&self) -> Block {
        let Dimensions { width, height } = self.fill_size();
        let mut inner_block = Block::new(width as usize, height as usize, self.style.background);

        match &self.content {
            Content::Text(text, alignment) => draw_text(
                &mut inner_block,
                text,
                *alignment,
                &self.style.font,
                self.style.foreground,
                self.style.background,
            ),
            Content::Paragraph(wrapped, alignment) => {
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
                        *alignment,
                        &self.style.font,
                        self.style.foreground,
                        self.style.background,
                    );
                    let line_block_height = line_block.height;
                    inner_block.paint(line_block, 0, y);
                    y += line_block_height; // FIXME
                    if y > height as usize {
                        break;
                    }
                }
            }
            Content::Row(children) => {
                let element_size = self.overall_size();
                let children_width: u32 = children
                    .iter()
                    .map(|child| child.overall_size().width)
                    .sum();
                let children_height: u32 = children
                    .iter()
                    .map(|child| child.overall_size().height)
                    .max()
                    .unwrap_or_default();
                let flex_room_hor = element_size.width.saturating_sub(children_width);
                let flex_room_ver = element_size.height.saturating_sub(children_height);
                let flexes_hor: u32 = children
                    .iter()
                    .map(|child| child.flex.horizontal_flexes() as u32)
                    .sum();
                let flexes_ver: u32 = children
                    .iter()
                    .map(|child| child.flex.vertical_flexes() as u32)
                    .sum();
                let room_per_flex_hor = flex_room_hor.checked_div(flexes_hor).unwrap_or_default();
                let room_per_flex_ver = flex_room_ver.checked_div(flexes_ver).unwrap_or_default();

                let mut x = 0;
                for child in children {
                    if child.flex.left {
                        x += room_per_flex_hor
                    }
                    inner_block.paint(
                        child.block(),
                        x as usize,
                        child.flex.top as usize * room_per_flex_ver as usize,
                    );
                    if child.flex.right {
                        x += room_per_flex_hor
                    }
                    x += child.overall_size().width;
                }
            }
            Content::Stack(children) => {
                let element_size = self.overall_size();
                let children_width: u32 = children
                    .iter()
                    .map(|child| child.overall_size().width)
                    .max()
                    .unwrap_or_default();
                let children_height: u32 = children
                    .iter()
                    .map(|child| child.overall_size().height)
                    .sum();
                let flex_room_hor = element_size.width.saturating_sub(children_width);
                let flex_room_ver = element_size.height.saturating_sub(children_height);
                let flexes_hor: u32 = children
                    .iter()
                    .map(|child| child.flex.horizontal_flexes() as u32)
                    .sum();
                let flexes_ver: u32 = children
                    .iter()
                    .map(|child| child.flex.vertical_flexes() as u32)
                    .sum();
                let room_per_flex_hor = flex_room_hor.checked_div(flexes_hor).unwrap_or_default();
                let room_per_flex_ver = flex_room_ver.checked_div(flexes_ver).unwrap_or_default();

                let mut y = 0;
                for child in children {
                    if child.flex.top {
                        y += room_per_flex_ver
                    }
                    inner_block.paint(
                        child.block(),
                        child.flex.left as usize * room_per_flex_hor as usize,
                        y as usize,
                    );
                    if child.flex.bottom {
                        y += room_per_flex_ver
                    }
                    y += child.overall_size().height;
                }
            }
        }

        let Dimensions { width, height } = self.overall_size();
        let mut padded_block = Block::new(width as usize, height as usize, self.style.background);
        padded_block.paint(
            inner_block,
            self.padding.left as usize,
            self.padding.top as usize,
        );
        padded_block
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
    let mut scrap = Block::new(scrap_width, font.height(), background);
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

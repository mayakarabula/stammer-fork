use fleck::Font;

/// A wrapper for a [`String`] where its contents are guaranteed to be wrapped at construction.
#[derive(Debug, Default, Clone)]
pub struct WrappedText(String, Vec<usize>);

impl WrappedText {
    /// Creates a new [`WrappedText`] that will be wrapped to the specified `width` and according
    /// to the glyphs in the provided [`Font`].
    pub fn new(text: String, width: u32, font: &Font) -> Self {
        Self::new_without_width(text, Some(width), font)
    }

    // TODO: Consider whether it is worth it to expose this function as `pub`. Will a user ever
    // actually need this, especially with a good builder API for the Element tree?
    // The function does not really do harm but also, it is quite an implementation detail. It will
    // make it more expensive to mess with it at a later time.
    /// Set up a new [`WrappedText`] without wrapping any lines.
    ///
    /// In order to wrap the text to the desired width at a later stage, call
    /// [`WrappedText::rewrap`].
    pub(crate) fn new_without_width(text: String, width: Option<u32>, font: &Font) -> Self {
        let mut ret = Self(text, Vec::new());
        ret.rewrap(width, font);
        ret
    }


    pub fn rewrap(&mut self, maxwidth: Option<u32>, font: &Font) {
        // TODO: Do this optimization that I had this note for:
        // > TODO: I don't know whether this makes any sense. Never measured it. I like it because
        // > it may prevent two allocations but also, who cares.

        // TODO: Equal starts optimization.

        let Self(text, breaklist) = self;
        breaklist.clear();
        let mut scrapwidth = 0u32;
        let mut wordwidth = 0u32;
        // FIXME: There may be a bug with a very long unbroken first line because we set it to 0
        // here. Maybe consider a None here.
        let mut last_whitespace = None;
        for (idx, ch) in text.char_indices() {
            match ch {
                '\n' => {
                    scrapwidth = 0;
                    wordwidth = 0;
                    last_whitespace = None; // FIXME: Or None?
                    breaklist.push(idx)
                }
                ch if maxwidth.is_some() => {
                    if ch.is_whitespace() {
                        last_whitespace = Some(idx);
                        wordwidth = 0;
                    }
                    let glyphwidth = font.glyph(ch).map_or(0, |ch| ch.width) as u32;
                    // TODO: Think about this unwrap().
                    if scrapwidth + glyphwidth > maxwidth.unwrap() {
                        let br = match last_whitespace {
                            Some(br) => br,
                            None => {
                                wordwidth = 0;
                                idx
                            }
                        };
                        breaklist.push(br);
                        wordwidth += glyphwidth;
                        scrapwidth = wordwidth;
                    } else {
                        wordwidth += glyphwidth;
                        scrapwidth += glyphwidth;
                    }
                }
                _ => {}
            }
        }

        breaklist.push(text.len());
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        let mut runner = 0;
        self.1.iter().map(move |&breakpoint| {
            let a = &self.0[runner..breakpoint];
            runner = breakpoint;
            if a.chars().next().is_some_and(|ch| ch.is_whitespace()) {
                &a[1..]
            } else {
                a
            }
        })
    }

    /// Returns the number of wrapped lines in this [`WrappedText`].
    pub fn lines_count(&self) -> usize {
        self.1.len()
    }

    /// Return a wrapped [`String`].
    ///
    /// It may be more efficient to use the [`WrappedText::lines`] directly, if that is actually what you need.
    pub fn wrapped(&self) -> String {
        self.lines().intersperse("\n").collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: Font = unsafe { std::mem::transmute(*include_bytes!("../../cream12.uf2")) };

    #[test]
    fn minimal() {
        let text = "hello dear\nworld".to_string();
        let enough_width = WrappedText::new(text.clone(), 200, &FONT);
        assert_eq!(enough_width.wrapped(), text);
        let wrapped = WrappedText::new(text, 50, &FONT);
        assert_eq!(wrapped.wrapped(), "hello\ndear\nworld");
    }

    #[test]
    fn lorem() {
        let lorem = include_str!("../../examples/lorem.txt").to_string();
        let wrapped = WrappedText::new(lorem, 300, &FONT);
        let correct = "Lorem ipsum dolor sit amet, officia excepteur ex\nfugiat reprehenderit \
            enim labore culpa sint ad\nnisi Lorem pariatur mollit ex esse exercitation\namet. \
            Nisi anim cupidatat excepteur officia.\nReprehenderit nostrud nostrud ipsum Lorem \
            est\naliquip amet voluptate voluptate dolor minim\nnulla est proident. Nostrud off\
            icia pariatur ut\nofficia. Sit irure elit esse ea nulla sunt ex\noccaecat reprehen\
            derit commodo officia dolor\nLorem duis laboris cupidatat officia voluptate.\nCulp\
            a proident adipisicing id nulla nisi laboris ex\nin Lorem sunt duis officia eiusmo\
            d. Aliqua\nreprehenderit commodo ex non excepteur duis\nsunt velit enim. Voluptate \
            laboris sint cupidatat\nullamco ut ea consectetur et est culpa et culpa\nduis.\n";
        assert_eq!(wrapped.wrapped(), correct);
    }

    #[test]
    fn short_lines() {
        let text = "This is some text
with some lines that
are obvious quite short.
In fact, they are much
shorter than 400 pixels.";
        let wrapped = WrappedText::new(text.to_string(), 400, &FONT);
        assert_eq!(wrapped.wrapped(), text);
    }

    #[test]
    fn long_lines() {
        let text = "Thequickbrownfoxjumpsoverthelazydog!!!!!
0123456789012345678901234567890123456789

abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ

";
        let correct = "Thequickbrownfoxjumpsoverthela
zydog!!!!!
012345678901234567890123456789
0123456789

abcdefghijklmnopqrstuvwxyzABCD
EFGHIJKLMNOPQRSTUVWXYZ

";
        let wrapped = WrappedText::new(text.to_string(), 200, &FONT);
        assert_eq!(wrapped.wrapped(), correct);
    }

    #[test]
    fn messy_whitespace() {
        // Quote from  Sadie Plant (1997), Zeros+Ones, p. 127.
        let text = r#"

Or does the error always come first? It was, after all, Grace
Hopper who, writing the software for the first electronic




programmable computer, introduced the terms "bug" and 
"debug" to computer programming when she found a



moth interrupting the smooth circuits of her new machine.



"#;
        let wrapped = WrappedText::new(text.to_string(), 300, &FONT);
        let correct = r#"

Or does the error always come first? It was, after
all, Grace
Hopper who, writing the software for the first
electronic




programmable computer, introduced the terms
"bug" and 
"debug" to computer programming when she
found a



moth interrupting the smooth circuits of her new
machine.



"#;
        assert_eq!(wrapped.wrapped(), correct);
    }

    #[test]
    fn lines_count() {
        let lorem = include_str!("../../examples/lorem.txt").to_string();
        let wrapped = WrappedText::new(lorem, 300, &FONT);
        assert_eq!(wrapped.lines_count(), wrapped.lines().count());
        assert_eq!(wrapped.lines_count(), 17);
    }

    #[test]
    fn rewrap() {
        let lorem = include_str!("../../examples/lorem.txt").to_string();
        let wrapped = WrappedText::new(lorem, 690, &FONT);
        let mut rewrapped = wrapped.clone();
        rewrapped.rewrap(Some(420), &FONT);
        rewrapped.rewrap(Some(690), &FONT);
        assert_eq!(
            wrapped.lines().collect::<Vec<_>>(),
            rewrapped.lines().collect::<Vec<_>>()
        );
    }
}

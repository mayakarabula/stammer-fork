use fleck::Font;

/// A wrapper for a [`String`] where its contents are guaranteed to be wrapped at construction.
#[derive(Debug, Default, Clone)]
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
        let linecount = text.lines().count(); // TODO: Is this very inefficient?
        for (n, line) in text.lines().enumerate() {
            // If it already would fit well, we don't need to do anything.
            if font.determine_width(line) <= width {
                wrapped.push_str(line);
                // Unless this is a last line that is not empty (which indicates it is just a
                // newline in the source text), push a newline to prepare for the next line.
                if n + 1 != linecount || line.is_empty() {
                    wrapped.push('\n');
                }
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

            wrapped.push_str(&scrap);
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

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: Font = unsafe { std::mem::transmute(*include_bytes!("../../cream12.uf2")) };

    #[test]
    fn minimal() {
        let text = "hello dear\nworld";
        let enough_width = WrappedText::new(text, 200, &FONT);
        assert_eq!(enough_width.as_ref(), text);
        let wrapped = WrappedText::new(text, 50, &FONT);
        assert_eq!(wrapped.as_ref(), "hello\ndear\nworld");
    }

    #[test]
    fn lorem() {
        let lorem = include_str!("../../examples/lorem.txt");
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
        assert_eq!(wrapped.as_ref(), correct);
    }

    #[test]
    fn short_lines() {
        let text = "This is some text
with some lines that
are obvious quite short.
In fact, they are much
shorter than 400 pixels.";
        let wrapped = WrappedText::new(text, 400, &FONT);
        assert_eq!(wrapped.as_ref(), text);
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
        let wrapped = WrappedText::new(text, 300, &FONT);
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
        assert_eq!(wrapped.as_ref(), correct);
    }
}

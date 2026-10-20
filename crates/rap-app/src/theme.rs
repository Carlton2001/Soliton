use gpui::{Rgba, rgb};

#[derive(Clone, Copy)]
pub struct Theme {
    pub background: Rgba,
    pub panel: Rgba,
    pub card: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub border: Rgba,
    pub accent: Rgba,
}

impl Theme {
    pub fn new(dark: bool) -> Self {
        if dark {
            Self {
                background: rgb(0x14171c),
                panel: rgb(0x191d24),
                card: rgb(0x222831),
                text: rgb(0xeef2f7),
                muted: rgb(0x9ba8b8),
                border: rgb(0x303944),
                accent: rgb(0x68d4b3),
            }
        } else {
            Self {
                background: rgb(0xf5f6f8),
                panel: rgb(0xffffff),
                card: rgb(0xe7edf1),
                text: rgb(0x202b37),
                muted: rgb(0x606e7c),
                border: rgb(0xd6dfe5),
                accent: rgb(0x147b65),
            }
        }
    }
}

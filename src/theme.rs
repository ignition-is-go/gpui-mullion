use gpui::{rgb, Hsla};
#[derive(Clone, Copy, Debug)]
pub struct MullionTheme {
    pub background: Hsla,
    pub surface: Hsla,
    pub border: Hsla,
    pub accent: Hsla,
    pub text: Hsla,
    pub muted_text: Hsla,
    pub focused: Hsla,
    pub drop_target: Hsla,
}
impl Default for MullionTheme {
    fn default() -> Self {
        Self {
            background: rgb(0x0e0e0e).into(),
            surface: rgb(0x151515).into(),
            border: rgb(0x303030).into(),
            accent: rgb(0x242424).into(),
            text: rgb(0xeeeeee).into(),
            muted_text: rgb(0x909090).into(),
            focused: rgb(0x62a0ea).into(),
            drop_target: rgb(0x355070).into(),
        }
    }
}

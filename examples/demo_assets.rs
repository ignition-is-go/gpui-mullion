use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

/// Material Design outlined icons embedded so native and browser demos render identically.
pub struct DemoAssets;

impl AssetSource for DemoAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let bytes: Option<&'static [u8]> = match path {
            "demo-icons/folder.svg" => Some(include_bytes!("assets/icons/folder.svg")),
            "demo-icons/description.svg" => Some(include_bytes!("assets/icons/description.svg")),
            "demo-icons/article.svg" => Some(include_bytes!("assets/icons/article.svg")),
            "demo-icons/timeline.svg" => Some(include_bytes!("assets/icons/timeline.svg")),
            "demo-icons/list.svg" => Some(include_bytes!("assets/icons/list.svg")),
            "demo-icons/edit_note.svg" => Some(include_bytes!("assets/icons/edit_note.svg")),
            "demo-icons/search.svg" => Some(include_bytes!("assets/icons/search.svg")),
            "demo-icons/find_replace.svg" => Some(include_bytes!("assets/icons/find_replace.svg")),
            "demo-icons/bookmarks.svg" => Some(include_bytes!("assets/icons/bookmarks.svg")),
            "demo-icons/code.svg" => Some(include_bytes!("assets/icons/code.svg")),
            "demo-icons/settings.svg" => Some(include_bytes!("assets/icons/settings.svg")),
            "demo-icons/palette.svg" => Some(include_bytes!("assets/icons/palette.svg")),
            "demo-icons/tune.svg" => Some(include_bytes!("assets/icons/tune.svg")),
            "demo-icons/keyboard.svg" => Some(include_bytes!("assets/icons/keyboard.svg")),
            "demo-icons/extension.svg" => Some(include_bytes!("assets/icons/extension.svg")),
            "demo-icons/apps.svg" => Some(include_bytes!("assets/icons/apps.svg")),
            _ => None,
        };
        Ok(bytes.map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        if path != "demo-icons" {
            return Ok(Vec::new());
        }
        Ok(vec![
            "folder.svg".into(),
            "description.svg".into(),
            "article.svg".into(),
            "timeline.svg".into(),
            "list.svg".into(),
            "edit_note.svg".into(),
            "search.svg".into(),
            "find_replace.svg".into(),
            "bookmarks.svg".into(),
            "code.svg".into(),
            "settings.svg".into(),
            "palette.svg".into(),
            "tune.svg".into(),
            "keyboard.svg".into(),
            "extension.svg".into(),
            "apps.svg".into(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_demo_icon_is_embedded_svg() {
        let assets = DemoAssets;
        let listed = assets.list("demo-icons").unwrap();
        assert_eq!(listed.len(), 16);
        for name in listed {
            let path = format!("demo-icons/{name}");
            let bytes = assets.load(&path).unwrap().expect("listed icon must load");
            let text = std::str::from_utf8(&bytes).expect("icon is UTF-8");
            assert!(text.contains("<svg"), "{path} is not SVG");
        }
    }

    #[test]
    fn unknown_demo_assets_are_not_claimed() {
        let assets = DemoAssets;
        assert!(assets.load("demo-icons/missing.svg").unwrap().is_none());
        assert!(assets.list("other").unwrap().is_empty());
    }
}

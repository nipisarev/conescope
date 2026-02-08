use std::borrow::Cow;

use gpui::{AssetSource, SharedString};
use rust_embed::RustEmbed;

#[derive(Debug, RustEmbed)]
#[folder = "assets"]
#[include = "icons/*.svg"]
pub struct ConescopeAssets;

impl AssetSource for ConescopeAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }
        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow::anyhow!("asset not found: {path}"))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| {
                if p.starts_with(path) {
                    Some(SharedString::from(p.to_string()))
                } else {
                    None
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_mapped_icons_exist() {
        let paths = [
            "icons/file-thin.svg",
            "icons/folder-simple-thin.svg",
            "icons/folder-open-thin.svg",
            "icons/file-rs-thin.svg",
            "icons/file-js-thin.svg",
            "icons/file-ts-thin.svg",
            "icons/file-jsx-thin.svg",
            "icons/file-tsx-thin.svg",
            "icons/file-py-thin.svg",
            "icons/file-c-thin.svg",
            "icons/file-cpp-thin.svg",
            "icons/file-c-sharp-thin.svg",
            "icons/file-html-thin.svg",
            "icons/file-css-thin.svg",
            "icons/file-sql-thin.svg",
            "icons/file-vue-thin.svg",
            "icons/file-md-thin.svg",
            "icons/image-thin.svg",
            "icons/video-camera-thin.svg",
            "icons/music-notes-thin.svg",
            "icons/file-code-thin.svg",
            "icons/file-text-thin.svg",
            "icons/file-archive-thin.svg",
            "icons/gear-thin.svg",
            "icons/plus-thin.svg",
            "icons/x-thin.svg",
            "icons/question-thin.svg",
            "icons/minus-circle-thin.svg",
            "icons/x-circle-thin.svg",
            "icons/squares-four-thin.svg",
            "icons/sidebar-simple-thin.svg",
            "icons/code-thin.svg",
            "icons/terminal-thin.svg",
            "icons/note-pencil-thin.svg",
            "icons/terminal-window-thin.svg",
            "icons/command-thin.svg",
            "icons/conescope-outline.svg",
            "icons/conescope-solid.svg",
        ];
        for path in paths {
            assert!(
                ConescopeAssets::get(path).is_some(),
                "missing asset: {path}"
            );
        }
    }
}

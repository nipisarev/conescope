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
            // File type icons (regular weight)
            "icons/file.svg",
            "icons/folder-simple.svg",
            "icons/folder-open.svg",
            "icons/file-rs.svg",
            "icons/file-js.svg",
            "icons/file-ts.svg",
            "icons/file-jsx.svg",
            "icons/file-tsx.svg",
            "icons/file-py.svg",
            "icons/file-c.svg",
            "icons/file-cpp.svg",
            "icons/file-c-sharp.svg",
            "icons/file-html.svg",
            "icons/file-css.svg",
            "icons/file-sql.svg",
            "icons/file-vue.svg",
            "icons/file-md.svg",
            "icons/image.svg",
            "icons/video-camera.svg",
            "icons/music-notes.svg",
            "icons/file-code.svg",
            "icons/file-text.svg",
            "icons/file-archive.svg",
            // UI icons (regular weight)
            "icons/gear.svg",
            "icons/plus.svg",
            "icons/x.svg",
            "icons/question.svg",
            "icons/minus-circle.svg",
            "icons/x-circle.svg",
            "icons/squares-four.svg",
            "icons/sidebar-simple.svg",
            "icons/note-pencil.svg",
            "icons/terminal-window.svg",
            "icons/command.svg",
            "icons/git-branch.svg",
            // Custom icons
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

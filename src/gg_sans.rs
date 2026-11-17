//! Discord's typeface, gg sans, which the window draws with once it has it.
//! Nothing of the font is in the repository: as Discord's web client does, the
//! app reads Discord's stylesheets for its files, then keeps them, decoded
//! from WOFF2, in the data folder's `fonts`. Until they're there, and without
//! a connection, DM Sans stands in. See docs/discord-look.md.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use slint::{ComponentHandle, fontique_012::fontique};
use yapfast_ui::{ClientStyle, YapfastWindow};

pub const FAMILY: &str = "gg sans";
/// Every weight Discord's stylesheets give, upright and italic.
const FILES: usize = 10;
/// The largest file kept: each weight is about 100 KiB decoded.
const MOST: usize = 4 << 20;

thread_local! {
    /// The windows to draw with it once it comes.
    static WINDOWS: RefCell<Vec<slint::Weak<YapfastWindow>>> = const { RefCell::new(Vec::new()) };
}

fn folder(root: &Path) -> PathBuf {
    root.join("fonts")
}

/// Registers each file kept that isn't yet; whether the font is there.
fn register(root: &Path) -> bool {
    static REGISTERED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    let Ok(files) = std::fs::read_dir(folder(root)) else {
        return false;
    };
    let files: Vec<PathBuf> = files
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("gg-sans-") && name.ends_with(".ttf"))
        })
        .collect();
    let mut registered = REGISTERED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    for path in files {
        if registered.contains(&path) {
            continue;
        }
        match std::fs::read(&path) {
            Ok(bytes) => {
                if crate::emoji_font::register_blob(FAMILY, fontique::Blob::new(Arc::new(bytes))) {
                    registered.push(path);
                }
            }
            Err(error) => tracing::warn!(%error, "could not read gg sans"),
        }
    }
    !registered.is_empty()
}

/// Draws `window` in gg sans if it's kept, and in it once it comes otherwise.
pub fn wear(window: &YapfastWindow, root: &Path) {
    if register(root) {
        window.global::<ClientStyle>().set_native_font(FAMILY.into());
    } else {
        WINDOWS.with_borrow_mut(|windows| windows.push(window.as_weak()));
    }
}

/// Fetches the font from Discord when it isn't kept yet, then draws the
/// windows waiting for it in it.
pub fn fetch(runtime: &yapfast_core::runtime::BackgroundRuntime, root: &Path) {
    let folder = folder(root);
    let kept = std::fs::read_dir(&folder)
        .map(|files| files.flatten().filter(|file| file.file_name().to_string_lossy().starts_with("gg-sans-")).count())
        .unwrap_or(0);
    if kept >= FILES {
        return;
    }
    let root = root.to_owned();
    let task = runtime.spawn(async move {
        if let Err(error) = download(&folder).await {
            tracing::warn!(error = format!("{error:#}"), "could not fetch gg sans from Discord");
            return;
        }
        let _ = slint::invoke_from_event_loop(move || {
            if register(&root) {
                WINDOWS.with_borrow_mut(|windows| {
                    for window in windows.drain(..).filter_map(|window| window.upgrade()) {
                        window.global::<ClientStyle>().set_native_font(FAMILY.into());
                    }
                });
            }
        });
    });
    // ponytail: the handle is leaked so the one fetch runs to its end, as
    // dropping it would cancel it; keep it on the adapter if it ever matters.
    std::mem::forget(task);
}

async fn download(folder: &Path) -> anyhow::Result<()> {
    use anyhow::Context;
    let client = crate::update::client().map_err(anyhow::Error::msg)?;
    let text = |url: String| {
        let client = client.clone();
        async move {
            let body = client
                .get(&url)
                .send()
                .await
                .and_then(|response| response.error_for_status())
                .map_err(|error| error.without_url())?
                .bytes()
                .await
                .map_err(|error| error.without_url())?;
            anyhow::Ok(body)
        }
    };
    let page = text("https://discord.com/app".to_owned()).await?;
    let page = String::from_utf8_lossy(&page);
    let sheets = stylesheets(&page);
    let mut faces = Vec::new();
    for sheet in sheets {
        let css = text(format!("https://discord.com{sheet}")).await?;
        faces.extend(font_faces(&String::from_utf8_lossy(&css)));
        if faces.len() >= FILES {
            break;
        }
    }
    anyhow::ensure!(!faces.is_empty(), "Discord's stylesheets name no gg sans");
    std::fs::create_dir_all(folder)?;
    for face in faces {
        let woff2 = text(format!("https://discord.com{}", face.url)).await?;
        anyhow::ensure!(woff2.len() <= MOST, "a gg sans file is too large");
        let mut packed: &[u8] = &woff2;
        let ttf = woff2_patched::decode::convert_woff2_to_ttf(&mut packed)
            .map_err(|error| anyhow::anyhow!("{error:?}"))
            .context("could not decode gg sans")?;
        let name = format!(
            "gg-sans-{}{}.ttf",
            face.weight,
            if face.italic { "-italic" } else { "" }
        );
        // Written whole, then named, so a file half written isn't taken.
        let part = folder.join(format!("{name}.part"));
        std::fs::write(&part, &ttf)?;
        std::fs::rename(&part, folder.join(name))?;
    }
    Ok(())
}

/// The addresses of the stylesheets a page links, as `/assets/….css`.
fn stylesheets(page: &str) -> Vec<String> {
    page.split("href=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .filter(|link| link.starts_with("/assets/") && link.ends_with(".css"))
        .map(str::to_owned)
        .collect()
}

#[derive(Debug, PartialEq)]
struct Face {
    weight: String,
    italic: bool,
    url: String,
}

/// gg sans's `@font-face` rules in a stylesheet: each file's weight, style
/// and address.
fn font_faces(css: &str) -> Vec<Face> {
    css.split("@font-face{")
        .skip(1)
        .filter_map(|rest| {
            let rule = rest.split('}').next()?;
            let value = |name: &str| {
                let start = rule.find(name)? + name.len();
                let value = rule[start..].split(';').next()?;
                Some(value.trim().trim_matches(['"', '\'']).to_owned())
            };
            if value("font-family:")? != FAMILY {
                return None;
            }
            let url = value("url(")?;
            let url = url.split(')').next()?.trim_matches(['"', '\'']).to_owned();
            url.starts_with("/assets/").then_some(())?;
            Some(Face {
                weight: value("font-weight:").unwrap_or_else(|| "400".into()),
                italic: value("font-style:").as_deref() == Some("italic"),
                url,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_discords_font_faces() {
        let css = r#"a{color:red}@font-face{font-family:"gg sans";font-weight:400;font-style:normal;src:url("/assets/66d7.woff2") format("woff2")}@font-face{font-family:"gg sans";font-weight:700;font-style:italic;src:url(/assets/ce3b.woff2) format("woff2")}@font-face{font-family:"Other";src:url(/assets/x.woff2)}"#;
        assert_eq!(
            font_faces(css),
            [
                Face { weight: "400".into(), italic: false, url: "/assets/66d7.woff2".into() },
                Face { weight: "700".into(), italic: true, url: "/assets/ce3b.woff2".into() },
            ]
        );
        let page = r#"<link rel="stylesheet" href="/assets/1.css"><link href="/assets/2.js"><a href="/x.css">"#;
        assert_eq!(stylesheets(page), ["/assets/1.css"]);
    }
}

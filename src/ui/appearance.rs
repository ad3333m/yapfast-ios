//! Appearance: choosing a theme, and making and editing the person's own.

use super::*;

impl Adapter {
    /// Shows the window in a theme, built in or the person's own, and keeps
    /// it chosen. False for an id Yapfast doesn't know, which changes nothing.
    pub(super) fn choose_theme(&mut self, window: &YapfastWindow, id: &str) -> bool {
        let Some((id, chosen)) =
            theme::lookup(id, &self.layout.custom_themes).map(|(id, theme)| (id.to_owned(), theme))
        else {
            return false;
        };
        window.set_theme_id(id.as_str().into());
        self.set_theme(window, &chosen);
        self.layout.theme = Some(id);
        self.layout.write(&self.root);
        true
    }

    /// Shows the window in another theme's colours, building again what
    /// carries the old mention colour in its markup: message rows, the
    /// topic, pins and search results.
    pub(super) fn set_theme(&mut self, window: &YapfastWindow, theme: &theme::Theme) {
        if theme.apply(window) {
            self.restyle_mentions();
            self.refresh(window);
        }
    }

    /// Marks for building again, at the next refresh, what carries the
    /// mention colour in its markup.
    pub(super) fn restyle_mentions(&mut self) {
        self.restyle = true;
        self.topic_shown = None;
        self.models.pins.clear();
        self.models.search_results.clear();
    }

    /// Opens the editor on a copy of the theme shown, which the person can
    /// swap for any other.
    pub(super) fn new_theme(&mut self, window: &YapfastWindow) {
        let current = window.get_theme_id();
        let starts: Vec<_> = theme::starts(&self.layout.custom_themes).collect();
        let index = starts
            .iter()
            .position(|start| start.id == current.as_str())
            .unwrap_or(0);
        let names: Vec<SharedString> = starts.iter().map(|start| start.name.into()).collect();
        let (base, start) = (starts[index].base, starts[index].theme);
        window.set_theme_name(copy_name(starts[index].name).into());
        window.set_theme_starts(ModelRc::new(VecModel::from(names)));
        window.set_theme_start(index as i32);
        self.open_theme_editor(window, theme::Editor::new(None, base, start));
    }

    /// Opens the editor on one of the person's own themes.
    pub(super) fn edit_theme(&mut self, window: &YapfastWindow, id: &str) {
        let Some(custom) = self
            .layout
            .custom_themes
            .iter()
            .find(|custom| custom.id == id)
        else {
            return;
        };
        window.set_theme_name(custom.name.as_str().into());
        let editor = theme::Editor::new(Some(custom.id.clone()), custom.base, custom.theme);
        self.open_theme_editor(window, editor);
    }

    fn open_theme_editor(&mut self, window: &YapfastWindow, editor: theme::Editor) {
        window.set_theme_editing_saved(editor.editing.is_some());
        window.set_theme_dark(editor.theme.dark);
        window.set_theme_colours(editor.rows());
        window.set_theme_can_save(true);
        window.set_theme_editor(true);
        self.set_theme(window, &editor.theme);
        self.theme_editor = Some(editor);
    }

    /// The new theme starts again from another, and takes its name too
    /// unless the person named it.
    pub(super) fn theme_start_chosen(&mut self, window: &YapfastWindow, index: i32) {
        let custom = &self.layout.custom_themes;
        let start = |index: i32| {
            let index = usize::try_from(index).ok()?;
            theme::starts(custom)
                .nth(index)
                .map(|start| (start.base, start.theme, copy_name(start.name)))
        };
        let (Some(editor), Some((base, theme, name))) = (self.theme_editor.as_mut(), start(index))
        else {
            return;
        };
        let named = start(window.get_theme_start())
            .is_none_or(|(_, _, before)| before != window.get_theme_name().as_str());
        if !named {
            window.set_theme_name(name.into());
        }
        editor.restart(base, theme);
        window.set_theme_start(index);
        window.set_theme_dark(theme.dark);
        window.set_theme_can_save(true);
        self.set_theme(window, &theme);
    }

    /// A colour's field changed: the window shows it at once if it's a
    /// colour.
    pub(super) fn theme_colour_edited(&mut self, window: &YapfastWindow, index: i32, text: &str) {
        let Some(editor) = self.theme_editor.as_mut() else {
            return;
        };
        let changed = usize::try_from(index).is_ok_and(|index| editor.edit(index, text));
        window.set_theme_can_save(editor.valid());
        if changed {
            let theme = editor.theme;
            self.set_theme(window, &theme);
        }
    }

    pub(super) fn theme_dark_toggled(&mut self, window: &YapfastWindow, dark: bool) {
        let Some(editor) = self.theme_editor.as_mut() else {
            return;
        };
        editor.theme.dark = dark;
        let theme = editor.theme;
        self.set_theme(window, &theme);
    }

    /// Keeps the theme as edited, among the person's own, and chooses it.
    pub(super) fn save_theme(&mut self, window: &YapfastWindow) {
        let Some(editor) = self.theme_editor.take_if(|editor| editor.valid()) else {
            return;
        };
        let name = match window.get_theme_name().trim() {
            "" => "My theme".to_owned(),
            name => name.to_owned(),
        };
        let custom = &mut self.layout.custom_themes;
        let saved = editor
            .editing
            .as_ref()
            .and_then(|id| custom.iter_mut().find(|custom| custom.id == *id));
        let id = match saved {
            Some(saved) => {
                saved.name = name;
                saved.theme = editor.theme;
                saved.id.clone()
            }
            None => {
                let id = theme::new_id(custom);
                custom.push(theme::Custom {
                    id: id.clone(),
                    name,
                    base: editor.base,
                    theme: editor.theme,
                });
                id
            }
        };
        close_theme_editor(window);
        window.set_themes(theme::cards(&self.layout.custom_themes));
        self.choose_theme(window, &id);
    }

    /// Closes the editor and shows the theme from before it opened.
    pub(super) fn cancel_theme(&mut self, window: &YapfastWindow) {
        if self.theme_editor.take().is_some() {
            close_theme_editor(window);
            let (_, before) =
                theme::chosen(self.layout.theme.as_deref(), &self.layout.custom_themes);
            self.set_theme(window, &before);
        }
    }

    /// Deletes the theme being edited. If it was the chosen one, Yap shows.
    pub(super) fn delete_theme(&mut self, window: &YapfastWindow) {
        let Some(editor) = self.theme_editor.take() else {
            return;
        };
        close_theme_editor(window);
        if let Some(id) = editor.editing {
            self.layout.custom_themes.retain(|custom| custom.id != id);
        }
        window.set_themes(theme::cards(&self.layout.custom_themes));
        let (id, _) = theme::chosen(self.layout.theme.as_deref(), &self.layout.custom_themes);
        let id = id.to_owned();
        self.choose_theme(window, &id);
    }
}

impl Adapter {
    /// Opens a file dialog for an Endcord, Vencord or BetterDiscord theme, in
    /// Endcord's themes folder when there is one, and imports the file picked.
    pub(super) fn import_theme_file(&mut self) {
        let (weak, runtime) = (self.weak.clone(), self.runtime.clone());
        let folder = theme_folders().into_iter().find(|folder| folder.is_dir());
        self.dialog(async move {
            let mut dialog = rfd::AsyncFileDialog::new()
                .set_title("Import a theme")
                .add_filter("Themes", &["css"]);
            if let Some(folder) = folder {
                dialog = dialog.set_directory(folder);
            }
            let Some(file) = dialog.pick_file().await else {
                return;
            };
            let name = file.file_name();
            let name = name.split('.').next().unwrap_or(&name).to_owned();
            match std::fs::read_to_string(file.path()) {
                Ok(css) => import_css(weak, runtime, css, name).await,
                Err(error) => import_note(&weak, &format!("Yapfast could not read it: {error}")),
            }
        });
    }

    /// Imports the theme a link gives, as Endcord's Online Themes take them.
    pub(super) fn import_theme_link(&mut self, link: &str) {
        let link = link.trim().to_owned();
        // The adapter is borrowed here, so the note goes straight to the window.
        let note = |note: &str| {
            if let Some(window) = self.window.upgrade() {
                window.set_theme_import_note(note.into());
            }
        };
        if !link.starts_with("https://") {
            note("Paste a link that starts with https://.");
            return;
        }
        note("Importing…");
        let (weak, runtime) = (self.weak.clone(), self.runtime.clone());
        let name = link
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .and_then(|last| last.split('.').next())
            .unwrap_or("Theme")
            .to_owned();
        let task = runtime.spawn(fetch_sheets(vec![link]));
        let spawned = slint::spawn_local(async move {
            match task.await.unwrap_or_default().pop() {
                Some(css) => import_css(weak, runtime, css, name).await,
                None => import_note(&weak, "Yapfast could not download that theme."),
            }
        });
        if let Err(error) = spawned {
            tracing::error!(%error, "could not import a theme");
        }
    }
}

/// Where Endcord and the mods it's built on keep their themes, for the file
/// dialog to open in.
fn theme_folders() -> Vec<std::path::PathBuf> {
    let Some(roaming) = std::env::var_os("APPDATA").map(std::path::PathBuf::from) else {
        return Vec::new();
    };
    ["Endcord", "Equicord", "Vencord", "BetterDiscord"]
        .iter()
        .map(|mod_name| roaming.join(mod_name).join("themes"))
        .collect()
}

/// Reads a theme's colours, with those of the sheets it imports under its
/// own, and keeps it among the person's themes, chosen.
async fn import_css(
    weak: std::rc::Weak<RefCell<Adapter>>,
    runtime: Arc<BackgroundRuntime>,
    css: String,
    name: String,
) {
    let imports = css_theme::imports(&css);
    let sheets = if imports.is_empty() {
        Vec::new()
    } else {
        runtime.spawn(fetch_sheets(imports)).await.unwrap_or_default()
    };
    let mut vars = css_theme::Vars::default();
    for sheet in &sheets {
        vars.read(sheet);
    }
    vars.read(&css);
    if css_theme::colours(&vars).is_empty() {
        import_note(&weak, "That theme sets no colours Yapfast can read.");
        return;
    }
    let name = css_theme::meta(&css, "name").unwrap_or(name);
    let Some(adapter) = weak.upgrade() else {
        return;
    };
    let mut adapter = adapter.borrow_mut();
    let Some(window) = adapter.window.upgrade() else {
        return;
    };
    let id = theme::new_id(&adapter.layout.custom_themes);
    let custom = theme::from_css(id.clone(), name.clone(), &vars);
    adapter.layout.custom_themes.push(custom);
    window.set_themes(theme::cards(&adapter.layout.custom_themes));
    adapter.choose_theme(&window, &id);
    window.set_theme_import_note(format!("Imported {name}. Its pencil edits its colours.").into());
}

/// Downloads each sheet, up to 8 of them and 4 MiB each; one that fails is
/// left out.
async fn fetch_sheets(links: Vec<String>) -> Vec<String> {
    let Ok(client) = crate::update::client() else {
        return Vec::new();
    };
    let mut sheets = Vec::new();
    for link in links.into_iter().take(8) {
        let response = client
            .get(&link)
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await
            .and_then(|response| response.error_for_status());
        match response {
            Ok(response) => match response.bytes().await {
                Ok(bytes) if bytes.len() <= 4 << 20 => {
                    sheets.push(String::from_utf8_lossy(&bytes).into_owned());
                }
                Ok(_) => tracing::warn!("a theme's sheet is over 4 MiB"),
                Err(error) => tracing::warn!(error = %error.without_url(), "could not download a theme's sheet"),
            },
            Err(error) => tracing::warn!(error = %error.without_url(), "could not download a theme's sheet"),
        }
    }
    sheets
}

/// Says under Appearance's themes how an import went.
fn import_note(weak: &std::rc::Weak<RefCell<Adapter>>, note: &str) {
    if let Some(window) = weak.upgrade().and_then(|adapter| adapter.borrow().window.upgrade()) {
        window.set_theme_import_note(note.into());
    }
}

/// What a new theme copied from another is called until the person names it.
fn copy_name(name: &str) -> String {
    format!("{name} copy")
}

/// Drops the editor's models from the window, so they're held only while
/// it's open.
fn close_theme_editor(window: &YapfastWindow) {
    window.set_theme_editor(false);
    window.set_theme_colours(ModelRc::default());
    window.set_theme_starts(ModelRc::default());
}

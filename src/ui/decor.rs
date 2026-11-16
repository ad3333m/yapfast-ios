//! EnhancedDecor, as the Endcord plugin of that name: any decoration or
//! nameplate from Discord's shop on the account's own avatar and name, shown
//! only here. `layout.json` keeps each account's choice by user id; the
//! Discord side wears it through `yapfast_discord::collectibles::set_local`.
//! See docs/enhanced-decor.md.

use super::*;

/// What EnhancedDecor puts on an account, as `layout.json` keeps it: SKUs,
/// and the nameplate's palette; empty for none.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct LocalLook {
    pub decoration: String,
    pub nameplate: String,
    pub palette: String,
}

/// The most of each kind the page lists at once; a search finds the rest.
const SHOWN: usize = 120;

thread_local! {
    /// The shop's decorations and nameplates, once fetched this run.
    static SHOP: RefCell<Option<Vec<yapfast_core::discord_link::ShopItem>>> =
        const { RefCell::new(None) };
    static QUERY: RefCell<String> = const { RefCell::new(String::new()) };
}

impl Adapter {
    /// Wears the account's EnhancedDecor look, once it's known.
    pub(super) fn wear_local_look(&self) {
        let Some(user) = self.app.state.discord_user.as_deref() else {
            return;
        };
        let look = self.layout.local_looks.get(user).cloned().unwrap_or_default();
        let some = |text: &str| Some(text.to_owned()).filter(|text| !text.is_empty());
        yapfast_core::discord_link::set_local_look(
            user,
            some(&look.decoration).as_deref(),
            some(&look.nameplate).as_deref().map(|sku| (sku, look.palette.as_str())),
        );
    }

    /// The page opened: fetches the shop the first time.
    pub(super) fn decor_opened(&mut self, window: &YapfastWindow) {
        if SHOP.with_borrow(Option::is_some) {
            self.show_decor(window);
            return;
        }
        window.set_decor_note("Loading Discord's shop…".into());
        let task = self.runtime.spawn(yapfast_core::discord_link::shop_catalog());
        let weak = self.weak.clone();
        let spawned = slint::spawn_local(async move {
            let found = task.await.map_err(|_| anyhow::anyhow!("stopped")).and_then(|found| found);
            let Some(adapter) = weak.upgrade() else {
                return;
            };
            let adapter = adapter.borrow();
            let Some(window) = adapter.window.upgrade() else {
                return;
            };
            match found {
                Ok(items) => {
                    SHOP.set(Some(items));
                    window.set_decor_note(SharedString::default());
                    adapter.show_decor(&window);
                }
                Err(error) => {
                    tracing::warn!(%error, "could not load Discord's shop");
                    window.set_decor_note(format!("Couldn't load Discord's shop: {error}").into());
                }
            }
        });
        if let Err(error) = spawned {
            tracing::error!(%error, "could not load Discord's shop");
        }
    }

    pub(super) fn decor_search(&self, window: &YapfastWindow, query: &str) {
        QUERY.set(query.trim().to_lowercase());
        self.show_decor(window);
    }

    fn show_decor(&self, window: &YapfastWindow) {
        let query = QUERY.with_borrow(Clone::clone);
        let (decorations, nameplates) = SHOP.with_borrow(|shop| {
            let found = shop
                .iter()
                .flatten()
                .filter(|item| query.is_empty() || item.name.to_lowercase().contains(&query));
            let row = |item: &yapfast_core::discord_link::ShopItem| DecorItem {
                sku: item.sku.to_string().into(),
                name: item.name.as_str().into(),
                picture_url: yapfast_core::discord_link::collectible_picture(item.sku).into(),
            };
            let decorations: Vec<_> = found.clone().filter(|item| !item.nameplate).take(SHOWN).map(row).collect();
            let nameplates: Vec<_> = found.filter(|item| item.nameplate).take(SHOWN).map(row).collect();
            (decorations, nameplates)
        });
        window.set_decor_decorations(ModelRc::new(VecModel::from(decorations)));
        window.set_decor_nameplates(ModelRc::new(VecModel::from(nameplates)));
        let look = self
            .app
            .state
            .discord_user
            .as_deref()
            .and_then(|user| self.layout.local_looks.get(user))
            .cloned()
            .unwrap_or_default();
        window.set_decor_decoration(look.decoration.into());
        window.set_decor_nameplate(look.nameplate.into());
    }

    /// Wears a decoration or nameplate, or takes it off when it's worn.
    pub(super) fn decor_choose(&mut self, window: &YapfastWindow, sku: &str, nameplate: bool) {
        let Some(user) = self.app.state.discord_user.clone() else {
            return;
        };
        let palette = SHOP.with_borrow(|shop| {
            shop.iter()
                .flatten()
                .find(|item| item.sku.to_string() == sku)
                .map(|item| item.palette.clone())
                .unwrap_or_default()
        });
        let look = self.layout.local_looks.entry(user).or_default();
        let slot = if nameplate { &mut look.nameplate } else { &mut look.decoration };
        *slot = if *slot == sku { String::new() } else { sku.to_owned() };
        if nameplate {
            look.palette = palette;
        }
        self.after_decor(window);
    }

    pub(super) fn decor_take_off(&mut self, window: &YapfastWindow) {
        if let Some(user) = self.app.state.discord_user.clone() {
            self.layout.local_looks.remove(&user);
        }
        self.after_decor(window);
    }

    fn after_decor(&mut self, window: &YapfastWindow) {
        self.layout.write(&self.root);
        self.wear_local_look();
        self.show_decor(window);
    }
}

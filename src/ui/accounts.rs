//! Switch Accounts, as Discord's account menu has it: each Discord account
//! linked on this computer, its login kept in the credential store by user id
//! and its name in `layout.json`. Switching links the other login and starts
//! again; Add an account unlinks this one here only, keeping its login.

use super::*;

/// A Discord account linked on this computer, as `layout.json` keeps it.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct KeptAccount {
    pub id: String,
    pub name: String,
    pub username: String,
    /// The address of its avatar; empty for none.
    pub avatar: String,
}

thread_local! {
    /// The account whose login this run kept, so it's kept once a run, after
    /// a new link too.
    static KEPT: RefCell<String> = const { RefCell::new(String::new()) };
}

impl Adapter {
    /// Keeps the account shown among those Switch Accounts lists.
    pub(super) fn keep_account(&mut self, window: &YapfastWindow) {
        let state = &self.app.state;
        let Some(id) = state.discord_user.clone() else {
            return;
        };
        if KEPT.with_borrow(|kept| *kept != id) {
            #[cfg(feature = "discord")]
            yapfast_core::discord_link::remember_account(&id);
            KEPT.set(id.clone());
            #[cfg(feature = "discord")]
            self.wear_local_look();
        }
        let account = KeptAccount {
            id: id.clone(),
            name: state.account_name.clone().unwrap_or_default(),
            username: state.account_username.clone().unwrap_or_default(),
            avatar: state.account_avatar.clone().unwrap_or_default(),
        };
        let accounts = &mut self.layout.discord_accounts;
        let changed = match accounts.iter_mut().find(|kept| kept.id == id) {
            Some(kept) if *kept == account => false,
            Some(kept) => {
                *kept = account;
                true
            }
            None => {
                accounts.push(account);
                true
            }
        };
        if changed {
            self.layout.write(&self.root);
        }
        if changed || window.get_saved_accounts().row_count() != self.layout.discord_accounts.len() {
            self.show_accounts(window);
        }
    }

    fn show_accounts(&self, window: &YapfastWindow) {
        let current = self.app.state.discord_user.as_deref();
        let rows: Vec<SwitchAccount> = self
            .layout
            .discord_accounts
            .iter()
            .map(|account| {
                let name = if account.name.is_empty() { &account.username } else { &account.name };
                SwitchAccount {
                    id: account.id.as_str().into(),
                    name: name.as_str().into(),
                    username: account.username.as_str().into(),
                    initial: initial(name),
                    tint: tint(name),
                    picture_url: account.avatar.as_str().into(),
                    current: current == Some(account.id.as_str()),
                }
            })
            .collect();
        window.set_saved_accounts(ModelRc::new(VecModel::from(rows)));
    }

    /// Links the account kept under `id` and starts again on it.
    pub(super) fn switch_account(&mut self, window: &YapfastWindow, id: &str) {
        #[cfg(feature = "discord")]
        match yapfast_core::discord_link::switch_account(id) {
            Ok(()) => window.invoke_restart(),
            Err(error) => {
                tracing::warn!(%error, "could not switch Discord accounts");
                self.app.state.notice = Some(format!("Couldn't switch accounts: {error}"));
                self.refresh(window);
            }
        }
    }

    /// Unlinks the account here only, keeping its login for switching back,
    /// and starts again at linking.
    pub(super) fn add_account(&mut self, window: &YapfastWindow) {
        #[cfg(feature = "discord")]
        match yapfast_core::discord_link::add_account() {
            Ok(()) => window.invoke_restart(),
            Err(error) => {
                tracing::warn!(%error, "could not set the Discord account aside");
                self.app.state.notice = Some(format!("Couldn't add an account: {error}"));
                self.refresh(window);
            }
        }
    }

    /// Signing out ends the account's login, so Switch Accounts drops it.
    pub(super) fn drop_signed_out_account(&mut self) {
        let Some(id) = self.app.state.discord_user.clone() else {
            return;
        };
        #[cfg(feature = "discord")]
        yapfast_core::discord_link::forget_account(&id);
        self.layout.discord_accounts.retain(|account| account.id != id);
        self.layout.write(&self.root);
    }
}

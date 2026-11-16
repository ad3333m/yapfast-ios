//! Linking a Discord account by QR code, or by a token the person pastes,
//! beside the chat backend, so a build without Matrix can link before
//! anything else runs. The login goes into the OS credential store; see
//! `yapfast_discord::remote_auth`.

use tokio::sync::mpsc::{self, UnboundedReceiver};
use yapfast_discord::{
    api::{Api, Refused},
    remote_auth::{self, Step, Stopped},
    token,
};

use crate::runtime::{BackgroundRuntime, BackgroundTask};

/// How linking goes, for [`crate::application::Application::linking`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// The code to show, as the address it carries.
    Code(String),
    /// Someone scanned it, by their username, and confirms on their phone.
    Scanned(String),
    /// Discord wants a captcha solved, on the page at this address.
    Captcha(String),
    /// The account is linked and its login saved.
    Linked,
    /// The code expired before anyone confirmed it.
    Expired,
    Failed(String),
}

/// Starts linking an account on `runtime`: by QR code, or with `token`
/// when the person pasted one. Its progress arrives on the receiver until it
/// ends, and dropping the task stops it.
pub fn start(
    runtime: &BackgroundRuntime,
    token: Option<String>,
) -> (BackgroundTask<()>, UnboundedReceiver<Progress>) {
    let (sender, progress) = mpsc::unbounded_channel();
    let report = move |step| {
        let _ = sender.send(step);
    };
    let task = match token {
        Some(token) => runtime.spawn(with_token(token, report)),
        None => runtime.spawn(run(report)),
    };
    (task, progress)
}

/// Links the account a pasted token belongs to, once Discord accepts it.
async fn with_token(token: String, progress: impl Fn(Progress)) {
    let token = match token::clean(&token) {
        Ok(token) => token,
        Err(error) => return progress(Progress::Failed(error.to_string())),
    };
    let checked = async {
        let api = Api::new(token.clone())?;
        api.learn_build_number().await;
        api.me().await?;
        token::store(&token)
    };
    progress(match checked.await {
        Ok(()) => Progress::Linked,
        Err(error) => match error.downcast_ref::<Refused>() {
            Some(Refused(status)) if status.as_u16() == 401 => {
                Progress::Failed("Discord didn't accept that token.".into())
            }
            _ => {
                tracing::warn!(
                    error = format!("{error:#}"),
                    "could not link Discord by token"
                );
                Progress::Failed(format!("Discord couldn't link Discord: {error}"))
            }
        },
    });
}

/// Links an account, telling `progress` how it goes until it ends.
async fn run(progress: impl Fn(Progress)) {
    let linked = remote_auth::link(|step| {
        progress(match step {
            Step::Code(address) => Progress::Code(address),
            Step::Scanned { name, .. } => Progress::Scanned(name),
            Step::Captcha(page) => Progress::Captcha(page),
        })
    })
    .await;
    progress(match linked.and_then(|login| token::store(&login)) {
        Ok(()) => Progress::Linked,
        Err(error) => match error.downcast_ref::<Stopped>() {
            Some(Stopped::Expired) => Progress::Expired,
            Some(stopped) => Progress::Failed(stopped.to_string()),
            None => {
                tracing::warn!(error = format!("{error:#}"), "could not link Discord");
                Progress::Failed(format!("Discord couldn't link Discord: {error}"))
            }
        },
    });
}

/// Switch Accounts: keeps the linked account's login under its user id.
pub fn remember_account(user: &str) {
    if let Err(error) = token::remember(user) {
        tracing::warn!(%error, "could not keep the Discord login for switching");
    }
}

/// Links the account kept under `user` instead; the app starts again on it.
pub fn switch_account(user: &str) -> anyhow::Result<()> {
    token::switch_to(user)
}

/// Unlinks the account here only, keeping its login, to link another.
pub fn add_account() -> anyhow::Result<()> {
    token::set_aside()
}

/// Forgets the login kept for `user`.
pub fn forget_account(user: &str) {
    if let Err(error) = token::drop_saved(user) {
        tracing::warn!(%error, "could not forget a kept Discord login");
    }
}

pub use yapfast_discord::collectibles::ShopItem;

/// The address of a decoration's or nameplate's still picture.
pub fn collectible_picture(sku: u64) -> String {
    yapfast_discord::collectibles::picture(sku)
}

/// EnhancedDecor: the shop's decorations and nameplates.
pub async fn shop_catalog() -> anyhow::Result<Vec<yapfast_discord::collectibles::ShopItem>> {
    let api = Api::new(token::load()?)?;
    Ok(yapfast_discord::collectibles::catalog(&api.collectibles_categories().await?))
}

/// EnhancedDecor: shows a decoration and nameplate on `user`'s own account
/// here, by SKU and the plate's palette; none of either shows what they equip.
pub fn set_local_look(user: &str, decoration: Option<&str>, nameplate: Option<(&str, &str)>) {
    use yapfast_discord::collectibles::{LocalLook, Plate, set_local};
    let look = LocalLook {
        decoration: decoration.and_then(|sku| sku.parse().ok()),
        plate: nameplate
            .and_then(|(sku, palette)| Some(Plate::restore(sku.parse().ok()?, palette))),
    };
    set_local(user, (look != LocalLook::default()).then_some(look));
}

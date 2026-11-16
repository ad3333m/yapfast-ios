//! The Translate plugin: a message's menu asks Google's free translation
//! endpoint for it in English, and the notice above the conversation shows
//! the answer. See docs/plugins.md.

use super::*;

impl Adapter {
    pub(super) fn translate(&mut self, item: &str) {
        let Some(text) = self.app.state.message(item).map(|message| message.body.clone()) else {
            return;
        };
        let task = self.runtime.spawn(translated(text));
        let weak = self.weak.clone();
        let spawned = slint::spawn_local(async move {
            let notice = match task.await {
                Ok(Ok((text, from))) => format!("Translated from {from}: {text}"),
                Ok(Err(error)) => format!("Couldn't translate that message: {error}"),
                Err(_) => "Couldn't translate that message.".to_owned(),
            };
            let Some(adapter) = weak.upgrade() else {
                return;
            };
            let mut adapter = adapter.borrow_mut();
            adapter.app.state.notice = Some(notice);
            if let Some(window) = adapter.window.upgrade() {
                adapter.refresh(&window);
            }
        });
        if let Err(error) = spawned {
            tracing::error!(%error, "could not start translating");
        }
    }
}

async fn translated(text: String) -> Result<(String, String), String> {
    let client = crate::update::client()?;
    let body = client
        .get(yapfast_core::plugins::translate_url(&text))
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.without_url().to_string())?
        .bytes()
        .await
        .map_err(|error| error.without_url().to_string())?;
    let answer: serde_json::Value =
        serde_json::from_slice(&body).map_err(|error| error.to_string())?;
    yapfast_core::plugins::translation(&answer).ok_or_else(|| "no translation came back".to_owned())
}

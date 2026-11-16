# Plugins

Settings → Plugins turns on and off behaviours in the manner of Vencord's
plugins, named as Vencord names them. The switches are the `Plugins` global in
`ui/theme.slint`; `layout.json` keeps the names of those on in `plugins`, and
`src/ui.rs` (`show_plugins`, `read_plugins`) hands them to
`crates/yapfast-core/src/plugins.rs`, which the code each one changes reads.

| Plugin | What it does | Where |
| --- | --- | --- |
| MessageLogger | A deleted message stays, with `-# (deleted)` under it, and an edited one shows what it said before, a `-# was:` line for each edit. Kept while Yapfast holds the channel's messages; loading it again from Discord drops them. Editing your own message leaves the lines out. | `change` in `discord_text/open_channel.rs`, `begin_edit` |
| FakeNitro | The emoji picker unlocks every server's emoji, animated ones too. A `:name:` this account can't use goes as a link to its picture whose text is a zero-width space, `[​](https://cdn.discordapp.com/emojis/ID.webp?…)`, so only the picture Discord embeds shows. Reactions still need Nitro. | `src/emoji.rs`, `outgoing_body` |
| SecretGuard | A message with what passes for a card number (13 to 16 digits from 3 to 6 that pass the Luhn check) or `password:`/`password =`/`password is` and a value isn't sent; a notice says why, and Enter again with it showing sends it. | `send_in` in `application/compose.rs` |
| VolumeBooster | People's and streams' volume sliders reach 400%, about eight times as loud. | `ui/voice.slint`, `volume::set` |
| CleanerDiscord | The composer's GIF and sticker buttons go. | `ui/yapfast.slint` |
| DisableCameras | The camera button in a call stays off. | `ui/voice.slint` |
| PinIcon | A pinned message shows a pin and "Pinned" under its text. | `ui/messages.slint` |
| Translate | A message's menu gets Translate: Google's free endpoint (`translate.googleapis.com`, no account) puts it in English in the notice above the conversation. | `src/ui/translate.rs` |
| HomeTyping | The DM button shows dots while someone types to you in a DM. | `ui/rail.slint`, `show_workspace` |
| KeyboardSounds | Each change to the message box plays a short synthesized click, on the default speakers. | `sounds::click`, `set_draft_in` |
| GoogleThat | `/google words`, sent, goes as `[words](https://www.google.com/search?q=words)`. | `send` in `application/compose.rs` |

Four more flip settings other sections show too: SilentTyping is Messages'
"share typing" off, NoReplyMention its reply mention off, ClearURLs its
tracking removal, and ShowHiddenChannels Discord's locked channels.

Endcord plugins that can't come across: those that patch Discord's web
client itself (CrashHandler, ConcatenatedModules, NewPluginsManager,
NoNitroUpsell, the *API plugins), and NoTrack, as Yapfast sends Discord no
analytics anyway. LoginWithQR, VoiceMessages and UnlimitedAccounts are
built in: linking by QR code, voice messages and Switch Accounts.

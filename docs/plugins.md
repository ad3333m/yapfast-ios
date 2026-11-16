# Plugins

Settings → Plugins turns on and off behaviours in the manner of Vencord's
plugins, named as Vencord names them. The switches are the `Plugins` global in
`ui/theme.slint`; `layout.json` keeps the names of those on in `plugins`, and
`src/ui.rs` (`show_plugins`, `read_plugins`) hands them to
`crates/yapfast-core/src/plugins.rs`, which the code each one changes reads.

| Plugin | What it does | Where |
| --- | --- | --- |
| MessageLogger | A deleted message stays, with `-# (deleted)` under it, and an edited one shows what it said before, a `-# was:` line for each edit. Kept while Yapfast holds the channel's messages; loading it again from Discord drops them. Editing your own message leaves the lines out. | `change` in `discord_text/open_channel.rs`, `begin_edit` |
| FakeNitro | The emoji picker unlocks every server's emoji, animated ones too. A `:name:` this account can't use goes as a link to its picture, `[name](https://cdn.discordapp.com/emojis/ID.webp?…)`, which Discord shows. Reactions still need Nitro. | `src/emoji.rs`, `outgoing_body` |
| SecretGuard | A message with what passes for a card number (13 to 16 digits from 3 to 6 that pass the Luhn check) or `password:`/`password =`/`password is` and a value isn't sent; a notice says why, and Enter again with it showing sends it. | `send_in` in `application/compose.rs` |
| VolumeBooster | People's and streams' volume sliders reach 400%, about eight times as loud. | `ui/voice.slint`, `volume::set` |

Four more flip settings other sections show too: SilentTyping is Messages'
"share typing" off, NoReplyMention its reply mention off, ClearURLs its
tracking removal, and ShowHiddenChannels Discord's locked channels.

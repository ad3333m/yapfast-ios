# Looking like Discord

## gg sans

The window draws in gg sans, Discord's own typeface, once it has it. The font
isn't in the repository: on the first start, `src/gg_sans.rs` reads
`https://discord.com/app` for its stylesheets, finds gg sans's `@font-face`
rules (every weight from 400 to 800, upright and italic), downloads the WOFF2
files they name, decodes them to TrueType with `woff2-patched`, and keeps them
in the data folder's `fonts`, as `gg-sans-<weight>[-italic].ttf`. Each start
registers the files kept with Slint's font collection under the family
`gg sans` and sets `ClientStyle.native-font` to it; a window open when they
first arrive switches over then. Without them, offline on the first start
say, DM Sans stands in, and the next start tries again.

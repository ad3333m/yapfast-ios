# Switch Accounts

The account menu's Switch Accounts lists every Discord account linked on this
computer, as Discord's does. Each account's login stays in the OS credential
store under `discord-token-<user id>` (`yapfast_discord::token::remember`),
and `layout.json` keeps its name, username and avatar in `discord_accounts`
(`src/ui/accounts.rs`).

- Choosing another account links its login in place of this one, drops what
  the cache held of this one, and starts the app again.
- Add an account unlinks this account on this computer only, keeping its
  login working for switching back, and starts again at linking.
- Signing out ends the login at Discord, so the account leaves the list.

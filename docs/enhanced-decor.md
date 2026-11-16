# EnhancedDecor

Settings → EnhancedDecor, after the Endcord plugin of that name, puts any
avatar decoration or nameplate from Discord's shop on your own account. Only
you see it, on this computer: nothing changes on Discord.

- The page fetches the shop once a run, `/collectibles-categories/v2`, and
  lists its decorations and nameplates (`yapfast_discord::collectibles::catalog`),
  the first 120 of each, or those whose names match the search.
- Choosing one wears it; choosing it again, or Take them off, goes back to
  what the account equips. `layout.json` keeps each account's choice in
  `local_looks`, by user id, so switching accounts switches looks.
- `collectibles::set_local` holds the look, and `Names::decoration_sku` and
  `Names::plate` give it for the account's own id, so messages, the member
  list and the account card all wear it. `Names::version` counts its changes.

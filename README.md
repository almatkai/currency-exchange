# Currency Exchange for Tezbar

Convert currencies interactively in Tezbar on Windows or macOS with selectable rate provider.

## Command

- **Currency Exchange** — type an expression like `10`, `1+2`, `5.5 in EUR`, `100 in JPY at 2023/01/15`, select base currency via dropdown, pin favorites, and view live conversion with rate date. Searching filters by code, country, or currency name. Historical rates via date picker action.

Provider preference (`exchangerate-api` default or `unirate`) selects the upstream API; `API Key` preference is required (get keys at `https://app.exchangerate-api.com/dashboard` or `https://unirateapi.com/dashboard`). Results are cached 24h per provider/date in `LocalStorage`.

## Compatibility

- Windows: built and exercised in Tezbar on Windows 11.
- macOS: uses only cross-platform `@raycast/api` and `node-fetch` (pure JS fetch) and retains the upstream macOS-compatible design.
- Linux: not currently declared.

## Local verification

```powershell
pnpm install
pnpm typecheck
pnpm test
pnpm build
```

## Credits

Adapted for Tezbar from Raycast's MIT-licensed `currency-exchange` extension at upstream commit `6734d85992290066fd31a7f5bee3cca3d3545a1a`. Original author `xeric` and contributors remain credited in `package.json`.

Native Rust V2 development port: [v2/README.md](v2/README.md).

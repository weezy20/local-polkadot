# Changelog
### [v0.1.1 - v0.2.0] 
- Added `--skip-pjs` | `--skip-polkadotjs` to skip download/running a polkadot-js instance. Handy when you already have a explorer running in your system
- Changes to the code structure to make room for future additions 
- Made `--tmp` exclusive to `--path` and `--fresh`

### [v0.2.0 - v0.2.1]
- Replace bun -> yarn as it's now a hard dependency for polkadot-js/apps

### [v0.2.1 - v0.2.2]
- Remove dependency on `cURL`. We use reqwest now.
- Concurrent downloads for `pjs` and `polkadot` exe.
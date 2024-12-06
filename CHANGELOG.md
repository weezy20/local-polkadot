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

### [v0.2.2 - v0.2.3]
- No functional changes. Update to include newer binary in crates.io

### [v0.2.3 - v0.3.0]
- Removed system dependency on unzip. The only requirement is to have `yarn` on your system PATH
- Removed restriction of not being able to use `--path` with `--tmp` or `--fresh`. 
  There's no reason why `--path` should not be able to work with the aforementioned flags.`--path --tmp` is equivalent to `--path --fresh` except the latter will not remove the directory at the end of the  process 
- `--tmp` without `--path` creates a dir in `/tmp` and is cleaned up at the end of the process.

### [v0.4.0]
- Improved cli user experience

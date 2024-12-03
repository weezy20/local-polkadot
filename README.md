# Local Polkadot

Have you ever tried interacting with the Polkadot network using `polkadot.js.org/apps` and found it to be frustratingly slow?

Too much load on the rpc server can lead to a bad user experience. 

Wish if there was a 1-click (or few keystrokes) worth of commands to quickly setup a local rpc-node + explorer and start submitting transactions or check the staking dashboard without lag? 

## Enter local-polkadot

Usually developers and power users run their own RPC node locally and also an instance of `polkadot-js/apps` to monitor chain activity and submit transactions with complete peace of mind.

This involves starting two terminals, compiling the polkadot binary locally, running it with the right options, then in a second terminal, opening polkadot-js explorer and running it to interact with the local chain running on port 9944.

`local-polkadot` does that for you! 

It is intentionally kept simple so that you may audit the code for yourself and comes with a few options

## Usage

![demo](.assets/demo.gif)

Install it using `cargo-install` or compile it 
```sh
cargo install local-polkadot
```

Run it
```
local-polkadot
```


And that's it! Visit http://localhost:3000/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/explorer in your browser and give it a couple of minutes to sync up with mainnet.

`local-polkadot` by default creates a directory called `.local-polkadot` in your home ($HOME) and downloads the latest `polkadot` and `polkadot-js/apps` releases. It then starts them both: `polkadot` on port 9944 and `apps` on port 3000. Use `ctrl-c` to terminate and clean up.

If you find yourself using `local-polkadot` often you'd want to keep your downloaded software updated.

This is where you use `--fresh`

```
local-polkadot --fresh
```
Would remove the `$HOME/.local-polkadot` folder, recreate it, and redownload the latest `polkadot` and `apps` source code. 

You can also use local polkadot as `local-polkadot --tmp` to remove `$HOME/.local-polkadot` after you're done.

If you're already running an explorer, you might want to pass in `--skip-pjs` or `--skip-polkadotjs` which makes sure to only run the polkadot node and not the explorer

## More about local-polkadot

This tool does nothing new that couldn't be done manually. I created this for myself as I found myself doing the steps manually many times.

If you don't want store artifacts in `$HOME/.local-polkadot` you're free to specify a `--path` where it will download its resources.

It specifically looks for files `pjs.zip` and `polkadot` in its working directory, finding which would result in skipping of download.


For now only `polkadot` is supported, but I plan on including support for `kusama` as well. 

This tool works because `warp sync` a feature of polkadot and substrate built chains that allows it to quickly sync up with the main network by downloading finality proofs instead of whole blocks which would make this a tediously long exercise.

Here's the commands it executes internally : 
It uses some dependencies that are required on your system: `unzip` and `yarn`.

```sh
# For explorer
yarn install;
yarn run start;
# For Polkadot
polkadot --chain polkadot --sync warp --rpc-methods Safe --tmp --rpc-port 9944 --rpc-cors all ... # and a few more
```

To see what's new checkout the [CHANGELOG](CHANGELOG.md)


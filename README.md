
Branch where data will be send to zeromq (PUB) - control with arguments --port --host .<br>
Defaultly publisher will use bind, if you want to connect publisher use:
```
--connect
```

Fetching trades data for openbook v2 using websocket. This is just proof of concept. Use on your own risk!.
Websocket can be and will be instable

#### to run
```
cargo run --bin openbookv2-printer -- --rpc-url <YourRPC> --market <Pubkey of market which you want to listen, default SOL-USDC>
```
or you can build with cargo build --release

#### TODO
 - [ ] option to use polling via getBlock rpc call ...
 - [ ] store data in db (redis,mongodb)
 - [ ] geyser plugin instead of websocket
 - [ ] handling of websocket 
 - [ ] print trades for all markets.
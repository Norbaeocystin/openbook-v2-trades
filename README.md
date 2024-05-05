
Fetching trades data for openbook v2 using websocket. This is just proof of concept. Use on your own risk!.
Websocket can be and will be instable

##### If you want to publish filled trades to zeromq and process afterwards use branch zeromq

#### to run
```
cargo run --bin openbookv2-printer -- --rpc-url <YourRPC> --market <Pubkey of market which you want to listen, default SOL-USDC>
```
or you can build with cargo build --release

### if you want to print all openbook markets:
```
cargo run --example market -- --rpc-url <YourRPCWhereYouCanMakeGPACalls>
```

#### TODO
 - [ ] option to use polling via getBlock rpc call ...
 - [ ] store data in db (redis,mongodb)
 - [ ] geyser plugin instead of websocket
 - [ ] handling of websocket 
 - [ ] print trades for all markets.
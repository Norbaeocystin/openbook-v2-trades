pid=$(ps -ef | grep -v grep | grep "openbookv2-printer" | awk '{print $2}')
kill -9 $pid

# prompt user for URL
echo "Enter the RPC URL: "
read rpc_url

cargo run --release --bin openbookv2-printer -- --rpc-url  $rpc_url --connect --market CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3 &>> ~/log/trade-reporter-openbookv2.log &
cargo run --release --bin openbookv2-printer -- --rpc-url  $rpc_url --connect --market Aj7ydi3rQ2qz5DnHLZLC91cVsVPmaRu2cUf5ZDfryq3T &>> ~/log/trade-reporter-openbookv2.log &
cargo run --release --bin openbookv2-printer -- --rpc-url  $rpc_url --connect --market Gio5iGZF9YVvhX6vwW3fZEfnPhtafseapaseGbAoiH9D &>> ~/log/trade-reporter-openbookv2.log &
cargo run --release --bin openbookv2-printer -- --rpc-url  $rpc_url --connect --market DBSZ24hqXS5o8djunrTzBsJUb1P8ZvBs1nng5rmZKsJt &>> ~/log/trade-reporter-openbookv2.log &
cargo run --release --bin openbookv2-printer -- --rpc-url  $rpc_url --connect --market Gudvr1FPgxKfnMoEEBXDgXWzmoavTY7nGC9TcdM4s3SP &>> ~/log/trade-reporter-openbookv2.log &
cargo run --release --bin openbookv2-printer -- --rpc-url  $rpc_url --connect --market 2ekKD6GQy9CPqyqZyFdERr14JcjD5QcJj7DbFfW23k4W &>> ~/log/trade-reporter-openbookv2.log &
cargo run --release --bin openbookv2-printer -- --rpc-url  $rpc_url --connect --market 7iDUNFiwpGjgFW5JmAjhVGXWdBfBXkc9ibFnxUrPNHjM &>> ~/log/trade-reporter-openbookv2.log &
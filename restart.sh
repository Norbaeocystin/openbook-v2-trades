pid=$(ps -ef | grep -v grep | grep "openbookv2-printer" | awk '{print $2}')
kill -9 $pid

# prompt user for URL
echo "Enter the RPC URL: "
read rpc_url

cargo run --release --bin openbookv2-printer -- --rpc-url  $rpc_url --market CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3 Aj7ydi3rQ2qz5DnHLZLC91cVsVPmaRu2cUf5ZDfryq3T Gio5iGZF9YVvhX6vwW3fZEfnPhtafseapaseGbAoiH9D DBSZ24hqXS5o8djunrTzBsJUb1P8ZvBs1nng5rmZKsJt Gudvr1FPgxKfnMoEEBXDgXWzmoavTY7nGC9TcdM4s3SP 2ekKD6GQy9CPqyqZyFdERr14JcjD5QcJj7DbFfW23k4W 7iDUNFiwpGjgFW5JmAjhVGXWdBfBXkc9ibFnxUrPNHjM &>> ~/log/trade-reporter-openbookv2.log &
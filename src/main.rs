use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_client::rpc_response::Response;
use solana_program::example_mocks::solana_sdk::transaction::Transaction;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::{EncodedTransaction, EncodedTransactionWithStatusMeta};
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

const JUPITER_PROGRAM: &str = "JUP2jxvXaqu7NQY1GmNF4m1vodw12LVXYxbFL2uJvfo";

// TODO get all jupiter transactions which are stable swaps
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    let ws_url = "wss://api.mainnet-beta.solana.com".to_string();
    let client = solana_client::rpc_client::RpcClient::new(rpc_url);
    let hash = client.get_latest_blockhash()?;
    println!("Latest blockhash: {}", hash);
    let (sub, recv) = solana_client::pubsub_client::PubsubClient::logs_subscribe(
        ws_url.as_str(),
        RpcTransactionLogsFilter::Mentions(vec![JUPITER_PROGRAM.to_string()]),
        // RpcTransactionLogsFilter::All,
        RpcTransactionLogsConfig {
            commitment: Some((CommitmentConfig::finalized())),
        },
    )?;

    loop {
        sleep(Duration::from_secs(3));
        let sig = recv.recv()?.value.signature;
        let sig = Signature::from_str(&sig)?;
        let tx = client
            .get_transaction(
                &sig,
                solana_transaction_status::UiTransactionEncoding::JsonParsed,
            )?
            .transaction;
        if !filter(tx.clone()) {
            println!("{:?}", tx);
        }
    }

    Ok(())
}

// check if all token transfer instructions involve stable coins
pub fn filter(tx: EncodedTransactionWithStatusMeta) -> bool {
    false
}

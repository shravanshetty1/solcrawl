use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};

use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;

use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, UiCompiledInstruction, UiInstruction,
    UiMessage,
};
use spl_token::instruction::TokenInstruction;
use std::error::Error;
use std::ops::Index;
use std::str::FromStr;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

const JUPITER_PROGRAM: &str = "JUP2jxvXaqu7NQY1GmNF4m1vodw12LVXYxbFL2uJvfo";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

// TODO get all jupiter transactions which are stable swaps
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    let ws_url = "wss://api.mainnet-beta.solana.com".to_string();
    let client = Arc::new(solana_client::rpc_client::RpcClient::new(rpc_url));
    let hash = client.get_latest_blockhash()?;
    println!("Latest blockhash: {}", hash);
    let (_sub, recv) = solana_client::pubsub_client::PubsubClient::logs_subscribe(
        ws_url.as_str(),
        // RpcTransactionLogsFilter::Mentions(vec![JUPITER_PROGRAM.to_string()]),
        RpcTransactionLogsFilter::All,
        RpcTransactionLogsConfig {
            commitment: Some(CommitmentConfig::finalized()),
        },
    )?;

    loop {
        sleep(Duration::from_secs(3));
        let sig = recv.recv()?.value.signature;
        let sig = Signature::from_str(&sig)?;
        let tx = client
            .get_transaction(&sig, solana_transaction_status::UiTransactionEncoding::Json)?
            .transaction;
        let swap_filter = SwapFilter {
            client: client.clone(),
            approved_mints: vec![],
        };
        if swap_filter.filter(tx.clone()) {
            println!("{:?}", tx);
        }
    }
}

pub struct SwapFilter {
    client: Arc<RpcClient>,
    approved_mints: Vec<String>,
}

impl SwapFilter {
    pub fn filter(&self, tx: EncodedTransactionWithStatusMeta) -> bool {
        self.try_filter(tx).unwrap_or(true)
    }

    pub fn try_filter(&self, tx: EncodedTransactionWithStatusMeta) -> Result<bool, Box<dyn Error>> {
        let mut account_keys: Vec<String> = Vec::new();
        if let EncodedTransaction::Json(tx) = tx.transaction {
            if let UiMessage::Raw(msg) = tx.message {
                account_keys = msg.account_keys
            }
        }
        if account_keys.is_empty() {
            return Ok(true);
        }

        let inner_instructions = tx
            .meta
            .ok_or("tx does not contain meta".to_string())?
            .inner_instructions
            .ok_or("tx does not contain inner instructions".to_string())?;

        // get all token_transfer instructions
        let mut token_transfer_instructions: Vec<UiCompiledInstruction> = Vec::new();
        for i in inner_instructions {
            for i in i.instructions {
                if let UiInstruction::Compiled(i) = i {
                    let prog_id: &String = account_keys.index(i.program_id_index as usize);
                    if prog_id.clone() == *TOKEN_PROGRAM {
                        let tok_instruction =
                            spl_token::instruction::TokenInstruction::unpack(i.data.as_bytes())?;
                        match tok_instruction {
                            TokenInstruction::Transfer { .. } => {
                                token_transfer_instructions.push(i)
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        if token_transfer_instructions.is_empty() {
            return Ok(true);
        }

        // TODO this can be optimised
        // if any of the token transfers are not a stable coin - filter
        for i in token_transfer_instructions {
            let src_index = i
                .accounts
                .first()
                .ok_or("incorrect number of accounts for token transfer instruction".to_string())?;
            let src: &String = account_keys.index(*src_index as usize);
            let src = self
                .client
                .get_token_account(&Pubkey::from_str(src.as_str())?)?
                .ok_or("could not find source token_account".to_string())?;
            if !self.approved_mints.contains(&src.mint) {
                return Ok(true);
            }

            let dst_index = i.accounts.index(1);
            let dst: &String = account_keys.index(*dst_index as usize);
            let dst = self
                .client
                .get_token_account(&Pubkey::from_str(dst.as_str())?)?
                .ok_or("could not find source token_account".to_string())?;
            if !self.approved_mints.contains(&dst.mint) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

use solana_client::rpc_client::RpcClient;

use solana_program::pubkey::Pubkey;

use crate::TransactionFilter;
use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, UiCompiledInstruction, UiInstruction,
    UiMessage, UiTransactionTokenBalance,
};
use spl_token::instruction::TokenInstruction;
use std::error::Error;
use std::mem::transmute;
use std::ops::Index;
use std::str::FromStr;
use std::sync::Arc;

pub struct JupiterSwapToken {
    pub approved_tokens: Vec<String>,
    pub token_program: String,
}

impl TransactionFilter for JupiterSwapToken {
    fn filter(&self, tx: EncodedTransactionWithStatusMeta) -> bool {
        self.try_filter(tx).unwrap_or(true)
    }
}

impl JupiterSwapToken {
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

        let tx_creator = account_keys.first().ok_or("could not get tx creator")?;

        let creator_token_balances = tx
            .meta
            .ok_or("tx does not contain metadata")?
            .pre_token_balances
            .ok_or("does not have pre token balances")?
            .into_iter()
            .filter(|t| {
                if t.mint.starts_with("Sol") {
                    return true;
                }

                if let Some(owner) = t.owner.clone() {
                    owner != *tx_creator
                } else {
                    true
                }
            })
            .collect::<Vec<UiTransactionTokenBalance>>();

        if creator_token_balances.len() != 2 {
            return Ok(true);
        }

        for tok in creator_token_balances {
            if !self.approved_tokens.contains(&tok.mint) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

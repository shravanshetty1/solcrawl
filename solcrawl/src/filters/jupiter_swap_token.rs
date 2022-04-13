use crate::TransactionFilter;
use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, UiMessage, UiTransactionTokenBalance,
};
use std::error::Error;
use std::ops::Index;

pub struct JupiterSwapToken {
    pub approved_tokens: Vec<String>,
    pub token_program: String,
}

impl TransactionFilter for JupiterSwapToken {
    fn filter(&self, tx: EncodedTransactionWithStatusMeta) -> bool {
        self.try_filter(tx).unwrap_or(true)
    }
}

// TODO improve algorithm currently swaps such as USDT > USDC > UST are being filtered out
impl JupiterSwapToken {
    pub fn try_filter(&self, tx: EncodedTransactionWithStatusMeta) -> Result<bool, Box<dyn Error>> {
        let mut account_keys: Vec<String> = Vec::new();
        if let EncodedTransaction::Json(tx) = tx.transaction {
            if let UiMessage::Raw(msg) = tx.message {
                account_keys = msg.account_keys
            }
        }

        let tx_creator = account_keys
            .first()
            .ok_or("could not get tx creator")?
            .clone();

        let meta = tx.meta.ok_or("tx does not contain metadata")?;
        let mut pre = meta
            .pre_token_balances
            .ok_or("does not have pre token balances")?
            .into_iter()
            .filter(|t| {
                if let Some(owner) = t.owner.clone() {
                    owner == *tx_creator
                } else {
                    false
                }
            })
            .filter(|t| self.approved_tokens.contains(&t.mint))
            .collect::<Vec<UiTransactionTokenBalance>>();
        pre.sort_by(|t1, t2| Ord::cmp(&t1.mint, &t2.mint));

        let mut post = meta
            .post_token_balances
            .ok_or("does not have post token balances")?
            .into_iter()
            .filter(|t| {
                if let Some(owner) = t.owner.clone() {
                    owner == *tx_creator
                } else {
                    false
                }
            })
            .filter(|t| self.approved_tokens.contains(&t.mint))
            .collect::<Vec<UiTransactionTokenBalance>>();
        post.sort_by(|t1, t2| Ord::cmp(&t1.mint, &t2.mint));

        if pre.len() != 2 || pre.len() != post.len() {
            return Err("unexpected token balances".into());
        }

        let mut input_index: usize = 1;
        let mut output_index: usize = 0;
        if pre.index(0).ui_token_amount.amount.parse::<u64>()?
            > post.index(0).ui_token_amount.amount.parse::<u64>()?
        {
            input_index = 0;
            output_index = 1;
        }

        let input_amount = pre
            .index(input_index)
            .ui_token_amount
            .amount
            .parse::<u64>()?
            .checked_sub(
                post.index(input_index)
                    .ui_token_amount
                    .amount
                    .parse::<u64>()?,
            )
            .ok_or("unexpected input amount")?;
        let output_amount = post
            .index(output_index)
            .ui_token_amount
            .amount
            .parse::<u64>()?
            .checked_sub(
                pre.index(output_index)
                    .ui_token_amount
                    .amount
                    .parse::<u64>()?,
            )
            .ok_or("unexpected output amount")?;

        if input_amount == 0 || output_amount == 0 {
            return Err("unexpected token amounts".into());
        }

        Ok(false)
    }
}

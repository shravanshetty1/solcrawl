use solana_client::rpc_client::RpcClient;

use solana_program::pubkey::Pubkey;

use crate::TransactionFilter;
use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, UiCompiledInstruction, UiInstruction,
    UiMessage,
};
use spl_token::instruction::TokenInstruction;
use std::error::Error;
use std::ops::Index;
use std::str::FromStr;
use std::sync::Arc;

pub struct JupiterSwapToken {
    pub client: Arc<RpcClient>,
    pub approved_tokens: Vec<String>,
    pub token_program: String,
}

impl TransactionFilter for JupiterSwapToken {
    fn filter(&self, tx: EncodedTransactionWithStatusMeta) -> bool {
        self.try_filter(tx).unwrap_or(true)
    }
}

// TODO add new method
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
                    if prog_id.clone() == *self.token_program {
                        let decoded_instruction = bs58::decode(i.data.clone())
                            .into_vec()
                            .map_err(|e| e.to_string())?;
                        let tok_instruction = spl_token::instruction::TokenInstruction::unpack(
                            decoded_instruction.as_slice(),
                        )?;
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
            if !self.approved_tokens.contains(&src.mint) {
                return Ok(true);
            }

            let dst_index = i.accounts.index(1);
            let dst: &String = account_keys.index(*dst_index as usize);
            let dst = self
                .client
                .get_token_account(&Pubkey::from_str(dst.as_str())?)?
                .ok_or("could not find source token_account".to_string())?;
            if !self.approved_tokens.contains(&dst.mint) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

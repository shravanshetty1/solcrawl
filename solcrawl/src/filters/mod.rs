use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

pub mod jupiter_swap_token;

pub trait TransactionFilter {
    fn filter(&self, tx: &EncodedConfirmedTransactionWithStatusMeta) -> bool;
}

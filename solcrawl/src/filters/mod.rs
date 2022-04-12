use solana_transaction_status::EncodedTransactionWithStatusMeta;

pub mod jupiter_swap_token;

pub trait TransactionFilter {
    fn filter(&self, tx: EncodedTransactionWithStatusMeta) -> bool;
}

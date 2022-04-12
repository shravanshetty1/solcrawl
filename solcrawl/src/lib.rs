use crate::filters::TransactionFilter;
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::EncodedTransactionWithStatusMeta;
use std::error::Error;
use std::str::FromStr;

pub mod filters;

pub struct Crawler {
    rpc_url: String,
    ws_url: String,
    program_addr: String,
    filters: Vec<Box<dyn TransactionFilter>>,
    publisher: crossbeam::channel::Sender<(String, EncodedTransactionWithStatusMeta)>,
}

// TODO dont print to std out - use a logger
// TODO add retry rpc client
// TODO add rate limit
impl Crawler {
    pub fn new(
        program_addr: String,
        rpc_url: String,
        ws_url: String,
        filters: Vec<Box<dyn TransactionFilter>>,
    ) -> (
        Self,
        crossbeam::channel::Receiver<(String, EncodedTransactionWithStatusMeta)>,
    ) {
        let (publisher, tx_recv) = crossbeam::channel::unbounded();
        (
            Self {
                rpc_url,
                ws_url,
                program_addr,
                filters,
                publisher,
            },
            tx_recv,
        )
    }

    pub fn crawl(&self) {
        loop {
            let res = self.try_crawl();
            if let Err(e) = res {
                println!("crawl err - {}", e);
            }
        }
    }
    fn try_crawl(&self) -> Result<(), Box<dyn Error>> {
        let client = solana_client::rpc_client::RpcClient::new(self.rpc_url.clone());
        let (_sub, recv) = solana_client::pubsub_client::PubsubClient::logs_subscribe(
            self.ws_url.clone().as_str(),
            RpcTransactionLogsFilter::Mentions(vec![self.program_addr.clone()]),
            RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::finalized()),
            },
        )?;

        loop {
            let sig = recv.recv()?.value.signature;
            let sig = Signature::from_str(&sig)?;

            println!("{}", sig);

            let tx: EncodedTransactionWithStatusMeta;
            loop {
                let res = client
                    .get_transaction(&sig, solana_transaction_status::UiTransactionEncoding::Json);
                if let Ok(res) = res {
                    tx = res.transaction;
                    break;
                }
            }

            let mut should_filter = false;
            for filter in &self.filters {
                if filter.filter(tx.clone()) {
                    should_filter = true;
                    break;
                }
            }

            if !should_filter {
                self.publisher.send((sig.to_string(), tx))?;
            }
        }
    }
}

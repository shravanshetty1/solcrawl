use crate::filters::TransactionFilter;

use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};

use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::error::Error;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

pub struct WebSocketCrawler {
    rpc_url: String,
    ws_url: String,
    program_addr: String,
    filters: Vec<Box<dyn TransactionFilter>>,
    publisher: crossbeam::channel::Sender<(String, EncodedConfirmedTransactionWithStatusMeta)>,
    sleep_duration: Option<Duration>,
}

unsafe impl Send for WebSocketCrawler {}

// TODO dont print to std out - use a logger
impl WebSocketCrawler {
    pub fn new(
        program_addr: String,
        rpc_url: String,
        ws_url: String,
        filters: Vec<Box<dyn TransactionFilter>>,
        sleep_duration: Option<Duration>,
    ) -> (
        Self,
        crossbeam::channel::Receiver<(String, EncodedConfirmedTransactionWithStatusMeta)>,
    ) {
        let (publisher, tx_recv) = crossbeam::channel::unbounded();
        (
            Self {
                rpc_url,
                ws_url,
                program_addr,
                filters,
                publisher,
                sleep_duration,
            },
            tx_recv,
        )
    }

    pub fn crawl(&self) {
        loop {
            let res = self.try_crawl();
            if let Err(e) = res {
                println!("ws crawl err - {}", e);
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

            println!("ws crawler - {}", sig);

            let mut tx: Option<EncodedConfirmedTransactionWithStatusMeta> = None;
            for _ in 0..5 {
                if let Some(dur) = self.sleep_duration {
                    sleep(dur);
                }

                let res = client
                    .get_transaction(&sig, solana_transaction_status::UiTransactionEncoding::Json);
                if let Ok(res) = res {
                    tx = Some(res);
                    break;
                }
            }

            if let Some(tx) = tx {
                let mut should_filter = false;
                for filter in &self.filters {
                    if filter.filter(&tx) {
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
}

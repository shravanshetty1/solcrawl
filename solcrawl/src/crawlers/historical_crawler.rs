use crate::filters::TransactionFilter;

use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;

use solana_program::pubkey::Pubkey;

use solana_sdk::signature::Signature;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::error::Error;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

pub struct HistoricalCrawler {
    rpc_url: String,
    program_addr: String,
    filters: Vec<Box<dyn TransactionFilter>>,
    publisher: crossbeam::channel::Sender<(String, EncodedConfirmedTransactionWithStatusMeta)>,
    sleep_duration: Option<Duration>,
    curr_sig: Option<Signature>,
}

unsafe impl Send for HistoricalCrawler {}

// TODO dont print to std out - use a logger
impl HistoricalCrawler {
    pub fn new(
        program_addr: String,
        rpc_url: String,
        filters: Vec<Box<dyn TransactionFilter>>,
        sleep_duration: Option<Duration>,
        curr_sig_str: Option<String>,
    ) -> Result<
        (
            Self,
            crossbeam::channel::Receiver<(String, EncodedConfirmedTransactionWithStatusMeta)>,
        ),
        Box<dyn Error>,
    > {
        let mut curr_sig: Option<Signature> = None;
        if let Some(sig) = curr_sig_str {
            curr_sig = Some(Signature::from_str(sig.as_str())?)
        }

        let (publisher, tx_recv) = crossbeam::channel::unbounded();
        Ok((
            Self {
                rpc_url,
                program_addr,
                filters,
                publisher,
                sleep_duration,
                curr_sig,
            },
            tx_recv,
        ))
    }

    pub fn crawl(&mut self) {
        loop {
            let res = self.try_crawl();
            if let Err(e) = res {
                println!("hs crawl err - {}", e);
            }
        }
    }
    fn try_crawl(&mut self) -> Result<(), Box<dyn Error>> {
        let client = solana_client::rpc_client::RpcClient::new(self.rpc_url.clone());

        loop {
            let tx_statuses = client.get_signatures_for_address_with_config(
                &Pubkey::from_str(self.program_addr.as_str())?,
                GetConfirmedSignaturesForAddress2Config {
                    before: self.curr_sig,
                    until: None,
                    limit: None,
                    commitment: None,
                },
            )?;
            for tx_status in tx_statuses {
                let sig = Signature::from_str(tx_status.signature.as_str())?;
                self.curr_sig = Some(sig);

                println!("hs crawler - {}", sig);

                let mut tx: Option<EncodedConfirmedTransactionWithStatusMeta> = None;
                for _ in 0..5 {
                    if let Some(dur) = self.sleep_duration {
                        sleep(dur);
                    }

                    let res = client.get_transaction(
                        &sig,
                        solana_transaction_status::UiTransactionEncoding::Json,
                    );
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
}

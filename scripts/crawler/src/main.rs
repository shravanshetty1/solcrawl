use solcrawl::filters::jupiter_swap_token::JupiterSwapToken;

use crate::storage::models::tx::Tx;
use diesel::prelude::*;
use std::error::Error;
use std::time::Duration;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

pub mod handle_txs;
pub mod storage;

const RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const WS_URL: &str = "wss://api.mainnet-beta.solana.com";

const JUPITER_PROGRAM: &str = "JUP2jxvXaqu7NQY1GmNF4m1vodw12LVXYxbFL2uJvfo";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
const UST_MINT: &str = "9vMJfxuKxXBoEa7rM12mYLMwTacLMLDJqHozw96WQL8i";

diesel_migrations::embed_migrations!();

// crawling jupiter for stable swaps
fn main() -> Result<(), Box<dyn Error>> {
    let conn = storage::conn::establish_connection()?;
    embedded_migrations::run(&conn)?;

    let mut curr_sig: Option<String> = None;
    let res = crate::storage::schema::tx::table
        .order(crate::storage::schema::tx::block_time.asc())
        .first::<Tx>(&conn);
    if let Ok(tx) = res {
        curr_sig = Some(tx.sig)
    }

    println!("curr_sig - {:?}", curr_sig);

    let approved_tokens = vec![
        USDC_MINT.to_string(),
        USDT_MINT.to_string(),
        UST_MINT.to_string(),
    ];
    let swap_filter = Box::new(JupiterSwapToken {
        approved_tokens: approved_tokens.clone(),
        token_program: TOKEN_PROGRAM.to_string(),
    });

    let (ws_crawler, ws_recv) = solcrawl::crawlers::websocket_crawler::WebSocketCrawler::new(
        JUPITER_PROGRAM.to_string(),
        RPC_URL.to_string(),
        WS_URL.to_string(),
        vec![swap_filter.clone()],
        None,
    );

    let (mut h_crawler, h_recv) = solcrawl::crawlers::historical_crawler::HistoricalCrawler::new(
        JUPITER_PROGRAM.to_string(),
        RPC_URL.to_string(),
        vec![swap_filter],
        None,
        curr_sig,
    )?;

    std::thread::spawn(move || ws_crawler.crawl());
    std::thread::spawn(move || h_crawler.crawl());

    println!("started crawling, please wait - establishing web socket connection (this can take upto 20 seconds)");
    crate::handle_txs::handle_txs(&approved_tokens, conn, vec![ws_recv, h_recv]);

    Ok(())
}

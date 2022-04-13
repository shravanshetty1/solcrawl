use solcrawl::filters::jupiter_swap_token::JupiterSwapToken;

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
    let approved_tokens = vec![
        USDC_MINT.to_string(),
        USDT_MINT.to_string(),
        UST_MINT.to_string(),
    ];
    let swap_filter = Box::new(JupiterSwapToken {
        approved_tokens: approved_tokens.clone(),
        token_program: TOKEN_PROGRAM.to_string(),
    });

    let (crawler, recv) = solcrawl::Crawler::new(
        JUPITER_PROGRAM.to_string(),
        RPC_URL.to_string(),
        WS_URL.to_string(),
        vec![swap_filter],
        Some(Duration::from_millis(500)),
    );

    let conn = storage::conn::establish_connection()?;
    embedded_migrations::run(&conn)?;
    std::thread::spawn(move || crate::handle_txs::handle_txs(&approved_tokens, conn, recv));

    println!("started crawling, please wait - establishing web socket connection (this can take upto 20 seconds)");
    crawler.crawl();

    Ok(())
}

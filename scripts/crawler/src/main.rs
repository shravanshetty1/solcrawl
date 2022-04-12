use solana_client::rpc_client::RpcClient;
use solcrawl::filters::jupiter_swap_token::JupiterSwapToken;
use std::error::Error;
use std::sync::Arc;

const RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const WS_URL: &str = "wss://api.mainnet-beta.solana.com";

const JUPITER_PROGRAM: &str = "JUP2jxvXaqu7NQY1GmNF4m1vodw12LVXYxbFL2uJvfo";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
const UST_MINT: &str = "9vMJfxuKxXBoEa7rM12mYLMwTacLMLDJqHozw96WQL8i";

// TODO store in DB

// crawling jupiter for stable swaps
fn main() -> Result<(), Box<dyn Error>> {
    let swap_filter = Box::new(JupiterSwapToken {
        client: Arc::new(RpcClient::new(RPC_URL)),
        approved_tokens: vec![
            USDC_MINT.to_string(),
            USDT_MINT.to_string(),
            UST_MINT.to_string(),
        ],
        token_program: TOKEN_PROGRAM.to_string(),
    });

    let (crawler, recv) = solcrawl::Crawler::new(
        JUPITER_PROGRAM.to_string(),
        RPC_URL.to_string(),
        WS_URL.to_string(),
        vec![swap_filter],
    );

    std::thread::spawn(move || loop {
        let tx = recv.recv();
        if let Ok(tx) = tx {
            println!("{:?}", tx);
        }
    });

    crawler.crawl();

    Ok(())
}

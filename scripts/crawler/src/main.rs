use crate::models::create_tx::CreateTx;
use crossbeam::channel::Receiver;
use diesel::prelude::*;
use solana_client::rpc_client::RpcClient;
use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, UiMessage, UiTransactionTokenBalance,
    VersionedConfirmedBlock,
};
use solcrawl::filters::jupiter_swap_token::JupiterSwapToken;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Index;
use std::sync::Arc;

#[macro_use]
extern crate diesel;
extern crate dotenv;

pub mod conn;
pub mod models;
pub mod schema;

const RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const WS_URL: &str = "wss://api.mainnet-beta.solana.com";

const JUPITER_PROGRAM: &str = "JUP2jxvXaqu7NQY1GmNF4m1vodw12LVXYxbFL2uJvfo";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
const UST_MINT: &str = "9vMJfxuKxXBoEa7rM12mYLMwTacLMLDJqHozw96WQL8i";

// TODO store in DB
// TODO store amounts as float?

// crawling jupiter for stable swaps
fn main() -> Result<(), Box<dyn Error>> {
    let swap_filter = Box::new(JupiterSwapToken {
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
        let conn = crate::conn::establish_connection();
        loop {
            let res = handle_tx(&conn, recv.clone());
            if let Err(err) = res {
                println!("handle tx err - {}", err)
            }
        }
    });

    crawler.crawl();

    Ok(())
}

pub fn handle_tx(
    conn: &PgConnection,
    tx_recv: Receiver<(String, EncodedTransactionWithStatusMeta)>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let (sig, tx) = tx_recv.recv()?;
        let create_tx = build_create_tx_obj(sig, tx)?;

        println!("{:?}", create_tx);

        diesel::insert_into(schema::tx::table)
            .values(create_tx)
            .execute(conn)?;
    }
}

pub fn build_create_tx_obj(
    sig: String,
    tx: EncodedTransactionWithStatusMeta,
) -> Result<CreateTx, Box<dyn Error>> {
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
            if t.mint.starts_with("Sol") {
                return true;
            }

            if let Some(owner) = t.owner.clone() {
                owner != tx_creator
            } else {
                true
            }
        })
        .collect::<Vec<UiTransactionTokenBalance>>();
    pre.sort_by(|t1, t2| Ord::cmp(&t1.mint, &t2.mint));

    let mut post = meta
        .post_token_balances
        .ok_or("does not have post token balances")?
        .into_iter()
        .filter(|t| {
            if t.mint.starts_with("Sol") {
                return true;
            }

            if let Some(owner) = t.owner.clone() {
                owner != tx_creator
            } else {
                true
            }
        })
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

    let input_token = pre.index(input_index).mint.clone();
    let output_token = pre.index(output_index).mint.clone();
    let input_amount = post
        .index(input_index)
        .ui_token_amount
        .amount
        .parse::<u64>()?
        - pre
            .index(input_index)
            .ui_token_amount
            .amount
            .parse::<u64>()?;
    let output_amount = pre
        .index(output_index)
        .ui_token_amount
        .amount
        .parse::<u64>()?
        - post
            .index(output_index)
            .ui_token_amount
            .amount
            .parse::<u64>()?;

    Ok(CreateTx {
        sig,
        input_token,
        output_token,
        input_amount: input_amount as i64,
        output_amount: output_amount as i64,
    })
}

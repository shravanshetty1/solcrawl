use crossbeam::channel::Receiver;
use diesel::prelude::*;

use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, UiMessage, UiTransactionTokenBalance,
};

use crate::storage::models::create_tx::CreateTx;

use crate::storage::models::tx::Tx;
use std::error::Error;
use std::ops::Index;

pub fn handle_txs(
    approved_tokens: &Vec<String>,
    conn: PgConnection,
    recv: Receiver<(String, EncodedTransactionWithStatusMeta)>,
) {
    loop {
        let res = handle_tx(approved_tokens, &conn, recv.clone());
        if let Err(err) = res {
            println!("handle tx err - {}", err)
        }
    }
}

pub fn handle_tx(
    approved_tokens: &Vec<String>,
    conn: &PgConnection,
    tx_recv: Receiver<(String, EncodedTransactionWithStatusMeta)>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let (sig, tx) = tx_recv.recv()?;
        let create_tx = build_create_tx_obj(approved_tokens, sig.clone(), tx)?;

        let txs = crate::storage::schema::tx::table
            .filter(crate::storage::schema::tx::sig.eq(sig.as_str()))
            .load::<Tx>(conn)?;

        if !txs.is_empty() {
            println!("tx with sig already exists in database - {}", sig);
            continue;
        }

        println!("{:?}", create_tx);

        diesel::insert_into(crate::storage::schema::tx::table)
            .values(create_tx)
            .execute(conn)?;
    }
}

pub fn build_create_tx_obj(
    approved_tokens: &Vec<String>,
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
    let mut all_pre = meta
        .pre_token_balances
        .ok_or("does not have pre token balances")?
        .into_iter()
        .filter(|t| {
            if let Some(owner) = t.owner.clone() {
                owner == *tx_creator
            } else {
                false
            }
        })
        .collect::<Vec<UiTransactionTokenBalance>>();
    all_pre.sort_by(|t1, t2| Ord::cmp(&t1.mint, &t2.mint));

    let mut all_post = meta
        .post_token_balances
        .ok_or("does not have post token balances")?
        .into_iter()
        .filter(|t| {
            if let Some(owner) = t.owner.clone() {
                owner == *tx_creator
            } else {
                false
            }
        })
        .collect::<Vec<UiTransactionTokenBalance>>();
    all_post.sort_by(|t1, t2| Ord::cmp(&t1.mint, &t2.mint));

    if all_pre.len() != all_post.len() {
        return Err("unexpected token balances length".into());
    }

    let mut pre: Vec<UiTransactionTokenBalance> = Vec::new();
    let mut post: Vec<UiTransactionTokenBalance> = Vec::new();
    for i in 0..all_pre.len() {
        let diff = all_pre
            .index(i)
            .ui_token_amount
            .amount
            .parse::<u64>()?
            .abs_diff(all_post.index(i).ui_token_amount.amount.parse::<u64>()?);
        if diff > 0 {
            pre.push(all_pre.index(i).clone());
            post.push(all_post.index(i).clone());
        }
    }

    if pre.len() != 2 {
        return Err("unexpected token balances".into());
    }

    for tok in pre.iter() {
        if !approved_tokens.contains(&tok.mint) {
            return Err(format!("unexpected tok type - {}", tok.mint).into());
        }
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
    let input_amount = pre
        .index(input_index)
        .ui_token_amount
        .amount
        .parse::<u64>()?
        .checked_sub(
            post.index(input_index)
                .ui_token_amount
                .amount
                .parse::<u64>()?,
        )
        .ok_or("unexpected input amount")?;
    let output_amount = post
        .index(output_index)
        .ui_token_amount
        .amount
        .parse::<u64>()?
        .checked_sub(
            pre.index(output_index)
                .ui_token_amount
                .amount
                .parse::<u64>()?,
        )
        .ok_or("unexpected output amount")?;

    if input_amount == 0 || output_amount == 0 {
        return Err("unexpected token amounts".into());
    }

    Ok(CreateTx {
        sig,
        input_token,
        output_token,
        input_amount: input_amount as i64,
        output_amount: output_amount as i64,
    })
}

use crossbeam::channel::Receiver;
use diesel::prelude::*;

use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, UiMessage, UiTransactionTokenBalance,
};

use crate::storage::models::create_tx::CreateTx;

use std::error::Error;
use std::ops::Index;

pub fn handle_txs(recv: Receiver<(String, EncodedTransactionWithStatusMeta)>) {
    let conn = crate::storage::conn::establish_connection();
    loop {
        let res = handle_tx(&conn, recv.clone());
        if let Err(err) = res {
            println!("handle tx err - {}", err)
        }
    }
}

pub fn handle_tx(
    conn: &PgConnection,
    tx_recv: Receiver<(String, EncodedTransactionWithStatusMeta)>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let (sig, tx) = tx_recv.recv()?;
        let create_tx = build_create_tx_obj(sig, tx)?;

        println!("{:?}", create_tx);

        diesel::insert_into(crate::storage::schema::tx::table)
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
            if let Some(owner) = t.owner.clone() {
                owner == *tx_creator
            } else {
                false
            }
        })
        .collect::<Vec<UiTransactionTokenBalance>>();
    pre.sort_by(|t1, t2| Ord::cmp(&t1.mint, &t2.mint));

    let mut post = meta
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

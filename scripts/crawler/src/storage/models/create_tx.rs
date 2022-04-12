use crate::storage::schema::tx;

#[derive(Debug, Insertable)]
#[table_name = "tx"]
pub struct CreateTx {
    pub sig: String,
    pub input_token: String,
    pub output_token: String,
    pub input_amount: i64,
    pub output_amount: i64,
}

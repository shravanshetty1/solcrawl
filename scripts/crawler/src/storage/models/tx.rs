#[derive(Queryable)]
pub struct Tx {
    pub id: i32,
    pub sig: String,
    pub input_token: String,
    pub output_token: String,
    pub input_amount: i64,
    pub output_amount: i64,
    pub block_time: i64,
}

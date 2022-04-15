table! {
    tx (id) {
        id -> Int4,
        sig -> Varchar,
        input_token -> Varchar,
        output_token -> Varchar,
        input_amount -> Int8,
        output_amount -> Int8,
        block_time -> Int8,
    }
}

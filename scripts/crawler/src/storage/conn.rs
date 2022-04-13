use diesel::pg::PgConnection;
use diesel::prelude::*;

use std::env;
use std::error::Error;

pub fn establish_connection() -> Result<PgConnection, Box<dyn Error>> {
    dotenv::dotenv().ok();

    let database_url = env::var("DATABASE_URL")?;
    Ok(PgConnection::establish(&database_url)?)
}

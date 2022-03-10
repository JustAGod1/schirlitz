mod database;
mod bot;

use std::collections::hash_map::RandomState;
use std::env;
use std::option::Option::Some;

use futures::StreamExt;
use telegram_bot::*;
use telegram_bot::Api;
use crate::database::DatabaseAccessor;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let api = Api::new(token);

    let mut database = DatabaseAccessor::new();
    database.create_tables();

    let mut bot = bot::SchirlitzBot::new(api, database);
    bot.run().await;

    Ok(())
}


use std::time::Instant;

use rusqlite::{Connection, Error};
use time::OffsetDateTime;

const TABLE_NAME: &'static str = "jokes";

pub struct Joke {
    pub author: String,
    pub text: String,
    pub created_at: OffsetDateTime,
}

pub struct DatabaseAccessor {
    connection: Connection,
}

impl DatabaseAccessor {
    pub fn new() -> DatabaseAccessor {
        let connection = Connection::open("db.sql").unwrap();

        DatabaseAccessor {
            connection
        }
    }

    pub fn create_tables(&mut self) {
        self.connection
            .execute(
                format!("CREATE TABLE IF NOT EXISTS {} (\
                      id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,\
                      author varchar(256) NOT NULL,\
                      text varchar(1048576) NOT NULL,\
                      created_at datetime DEFAULT current_timestamp\
                  )", TABLE_NAME).as_str(),
                [],
            )
            .unwrap();
    }

    pub fn insert(&mut self, author: String, text: String) {
        self.connection.prepare(
            format!("INSERT INTO {} (author, text) VALUES (?, ?)", TABLE_NAME).as_str()
        )
            .unwrap()
            .insert([author, text])
            .unwrap();
    }


    pub fn query_jokes(&mut self, pattern: &str) -> Vec<Joke> {
        self.connection.prepare(format!("SELECT author, text, created_at FROM {} WHERE text LIKE '%{}%'", TABLE_NAME, pattern).as_str())
            .unwrap()
            .query_map([], |a| {
                Ok(Joke {
                    author: a.get(0).unwrap(),
                    text: a.get(1).unwrap(),
                    created_at: a.get(2).unwrap(),
                })
            }).unwrap().collect::<Result<Vec<Joke>, Error>>().unwrap()
    }
}
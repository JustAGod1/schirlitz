use std::collections::{HashMap, HashSet};

use futures::StreamExt;
use telegram_bot::{Api, CanAnswerInlineQuery, InlineQuery, InlineQueryResult, InlineQueryResultArticle, InputMessageContent, InputTextMessageContent, Message, MessageChat, MessageText, SendMessage, UpdateKind};

use crate::database::DatabaseAccessor;

pub struct SchirlitzBot {
    api: Api,
    database: DatabaseAccessor,
    waiting_from: HashMap<String, fn(&mut SchirlitzBot, Message)>,
}

impl SchirlitzBot {
    pub fn new(api: Api, database: DatabaseAccessor) -> Self {
        SchirlitzBot { api, database, waiting_from: HashMap::new() }
    }

    pub async fn run(&mut self) {
        let mut stream = self.api.stream();

        while let Some(event) = stream.next().await {
            let update = event.unwrap();
            println!("{:?}", update);
            match update.kind {
                UpdateKind::Message(msg) => {
                    self.process_message(msg);
                }
                UpdateKind::InlineQuery(query) => {
                    self.process_inline(query);
                }
                _ => {}
            }
        }
    }

    fn process_message(&mut self, msg: Message) {
        if msg.text().is_none() { return; }
        let text = msg.text().unwrap();

        let id = if let MessageChat::Private(user) = &msg.chat {
            user.id.to_string()
        } else {
            return;
        };

        if let Some(listener) = self.waiting_from.remove(&id) {
            listener(self, msg);
            return;
        }

        if text.starts_with("/add") {
            let req = SendMessage::new(msg.chat.clone(),
                                       "Отправьте анекдот\n\
                                            Можно разделять несколько анекдотов двумя переносами строки");
            self.waiting_from.insert(msg.chat.id().to_string(), Self::joke_sent);
            self.api.spawn(req);
        }
    }

    fn joke_sent(&mut self, msg: Message) {
        for joke in msg.text().unwrap().split("\n\n") {
            self.database.insert(msg.from.id.to_string(), joke.to_string());
        }
        let req = SendMessage::new(msg.chat.clone(), "Добавил!");
        self.api.spawn(req);
    }

    fn process_inline(&mut self, query: InlineQuery) {
        let jokes = self.database.query_jokes(&query.query);
        let answers: Vec<InlineQueryResult> = jokes.iter().enumerate().map(|(i, joke)| {
            InlineQueryResult::InlineQueryResultArticle(
                InlineQueryResultArticle {
                    id: i.to_string(),
                    title: joke.text.to_string(),
                    input_message_content: InputMessageContent::InputTextMessageContent(
                        InputTextMessageContent {
                            message_text: joke.text.to_string(),
                            parse_mode: None,
                            disable_web_page_preview: true,
                        }
                    ),
                    reply_markup: None,
                    url: None,
                    hide_url: false,
                    description: None,
                    thumb_url: None,
                    thumb_width: None,
                    thumb_height: None,
                }
            )
        }).collect();

        self.api.spawn(query.answer(answers))
    }
}

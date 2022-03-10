use std::collections::{HashMap, HashSet};
use std::process::Command;

use futures::StreamExt;
use rand::Rng;
use telegram_bot::{Api, CanAnswerInlineQuery, InlineQuery, InlineQueryResult, InlineQueryResultArticle, InputMessageContent, InputTextMessageContent, Message, MessageChat, MessageText, SendMessage, UpdateKind, User};
use crate::{Chat, UserId};

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
        if text.starts_with("/restart") {
            if msg.from.id != UserId(429171352) {
                self.api.spawn(SendMessage::new(chat, "Иди нахуй бесправный мудила"));
                return;
            }
            self.update(msg.chat);
        }
    }

    fn update(&mut self, chat: MessageChat) {
        self.api.spawn(SendMessage::new(chat.clone(), "Пулю"));

        let mut command = Command::new("git")
            .arg("pull");

        if !self.run_command(command, &chat) { return; }
        self.api.spawn(SendMessage::new(chat.clone(), "Перезапускаюсь"));

        let mut command = Command::new("sudo")
            .arg("systemctl")
            .arg("restart")
            .arg("bot");

        self.run_command(command, &chat);
    }

    fn run_command(&mut self, cmd: &mut Command, chat: &MessageChat) -> bool {
        return match cmd.output() {
            Ok(output) => {
                if output.status != 0 {
                    self.api.spawn(SendMessage::new(
                        chat,
                        format!(
                            "Процесс завершился с кодом {}:\n {}, {}",
                            output.status,
                            String::from_utf8_lossy(&output.stderr),
                            String::from_utf8_lossy(&output.stdout)
                        ),
                    ));
                    return false;
                }
                true
            }
            Err(err) => {
                self.api.spawn(SendMessage::new(chat, format!("Произошла ошибка: {}", err)));
                false
            }
        }
    }

    fn joke_sent(&mut self, msg: Message) {
        for joke in msg.text().unwrap().split("\n\n") {
            self.database.insert(msg.from.id.to_string(), joke.to_string());
        }
        let req = SendMessage::new(msg.chat.clone(), "Добавил!");
        self.api.spawn(req);
    }

    fn make_joke_answer(id: String, title: String, joke: String) -> InlineQueryResult {
        InlineQueryResult::InlineQueryResultArticle(
            InlineQueryResultArticle {
                id,
                title: joke.clone(),
                input_message_content: InputMessageContent::InputTextMessageContent(
                    InputTextMessageContent {
                        message_text: joke,
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
    }

    fn process_inline(&mut self, query: InlineQuery) {
        let jokes = self.database.query_jokes(query.query.as_str());
        let answers: Vec<InlineQueryResult>;
        if query.query.is_empty() {
            answers = vec![Self::make_joke_answer(
                "rand".to_string(),
                "Случайно".to_string(),
                jokes.get(rand::thread_rng().gen_range(0..jokes.len())).unwrap().text.clone()
            )]
        } else {
            answers = jokes.iter().enumerate().map(|(i, joke)| {
                Self::make_joke_answer(i.to_string(), joke.text.clone(), joke.text.clone());
            }).collect();
        }

        let mut answer = query.answer(answers);
        answer.cache_time(0);
        self.api.spawn(answer)
    }
}

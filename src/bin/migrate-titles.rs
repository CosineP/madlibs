extern crate madlibs;
#[macro_use]
extern crate serde_derive;

use madlibs::template;
use madlibs::bot;
use std::fs::File;
use std::collections::HashMap;
use bot::BotStatus;
use template::{Token, Template};

pub type OldTemplate = Vec<Token>;

#[derive(Deserialize, Serialize)]
pub struct OldBotStatus {
    pub last_noti_date: chrono::DateTime<chrono::Utc>,
    pub known_templates: Vec<OldTemplate>,
}

fn read_old() -> OldBotStatus {
    match File::open("status.bincode") {
        Ok(file) => {
            bincode::deserialize_from(file).expect("old status struct didn't match given db")
        },
        Err(_) => panic!("couldn't open old db file"),
    }
}

fn write(bot_status: BotStatus) {
    match File::create("status.bincode") {
        Ok(file) => {
            bincode::serialize_into(file, &bot_status)
                .expect("couldn't serialize to file (fs error, not schema)")
        },
        Err(_) => panic!("couldn't create/open status file")
    };
}

fn main() {
    let old = read_old();
    let mut new_templates = Vec::new();
    for plate in old.known_templates {
        new_templates.push(Template {
            title: None,
            body: plate,
        })
    }
    let new = BotStatus {
        known_templates: new_templates,
        last_noti_date: old.last_noti_date,
        collection_toots: HashMap::new(),
    };
    write(new);
}


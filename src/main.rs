// Mostly straight outta https://github.com/Aaronepower/Mammut README

extern crate elefren;
extern crate chrono;
extern crate toml;
#[macro_use]
extern crate serde_derive;

use elefren::{Mastodon, MastodonClient, Registration, StatusBuilder};
use elefren::helpers::cli;
use elefren::helpers;
use elefren::entities::*;

use std::fs::File;
use std::io::prelude::*;

mod madlibs;

#[derive(Deserialize, Serialize)]
struct BotStatus {
    last_noti_date: chrono::DateTime<chrono::Utc>,
    // known_templates: Vec<Template>, // TODO, when we start auto-posting
}

fn process_mention(mastodon: Mastodon, notification: notification::Notification) {
    let status = notification.status.unwrap();
    let text = status.content;
    let mut template = madlibs::to_template(text);
    let home = mastodon.get_home_timeline().expect("couldn't fetch home timeline");
    for status in home.items_iter() {
        if status.account.acct == "madlibs" || status.content.contains("madlibs") {
            continue;
        }
        match madlibs::reduce_template(&mut template, status.content) {
            Some(mut text) => {
                let end = format!("cc @{}", notification.account.acct);
                text.push_str(&end);
                mastodon.new_status(StatusBuilder {
                    status: text,
                    ..Default::default()
                }).expect("could not post status");
                break;
            },
            None => ()
        };
    }
}

fn process_follow(mastodon: Mastodon, account: account::Account) {
    let result = mastodon
        .follow(account.id.parse().expect("id is invalid"));
    match result {
        Ok(_) => (),
        Err(err) => println!("{}", err),
    };
}

fn sleep(secs: u64) {
    std::thread::sleep(std::time::Duration::from_secs(secs));
}

fn poll_loop(mastodon: Mastodon) {
    let sleep_time = 60; // in seconds

    let mut bot_status = match File::open("status.toml") {
        Ok(mut file) => {
            let mut string = String::new();
            file.read_to_string(&mut string).unwrap();
            toml::from_str(&string).unwrap()
        },
        Err(_) => BotStatus {
            // WARNING: Don't use this bot in the past
            last_noti_date: chrono::DateTime::from_utc(
                                chrono::naive::NaiveDateTime::from_timestamp(0, 0),
                                chrono::Utc)
        }
    };

    loop {
        println!("checking.......");
        let notis = mastodon.notifications().expect("couldn't fetch notis");
        let mut last_noti_date_temp = bot_status.last_noti_date;
        for noti in notis.initial_items {
            // If we have caught up with ourselves
            if noti.created_at <= bot_status.last_noti_date {
                // Exit, the loop is done, persistence is done outside of loop
                break;
            }
            // Only if we're on first run, getting the most recent noti
            if noti.created_at > last_noti_date_temp {
                last_noti_date_temp = noti.created_at;
            }

            match noti.notification_type {
                notification::NotificationType::Mention => process_mention(mastodon.clone(), noti),
                notification::NotificationType::Follow => process_follow(mastodon.clone(), noti.account),
                _ => (),
            };
        }
        // Now that we've finished our search, we can update our bot status
        bot_status.last_noti_date = last_noti_date_temp;
        // Serialize the bot status occasionally
        match File::create("status.toml") {
            Ok(mut file) => {
                let serialized = toml::to_string(&bot_status).unwrap();
                file.write_all(serialized.as_bytes())
                    .expect("couldn't write to status file");
            },
            Err(_) => panic!("couldn't create/open status file")
        };
        sleep(sleep_time);
    }
}

fn main() {
    let mastodon = match helpers::toml::from_file("credentials.toml") {
        Ok(data) => {
            Mastodon::from(data)
        }
        Err(_) => register(),
    };
    poll_loop(mastodon);
}

fn register() -> Mastodon {
    let registration = Registration::new("https://beeping.town")
        .client_name("madlibs-bot")
        .build()
        .unwrap();
    let mastodon = cli::authenticate(registration).unwrap();

    helpers::toml::to_file(&*mastodon, "credentials.toml")
        .expect("could not save credentials");

    mastodon
}


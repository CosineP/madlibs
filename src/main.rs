// Deals with the botty aspects: polling, sending, etc

extern crate elefren;
extern crate chrono;
extern crate toml;
// Yes, it is worth it for both. TOML doesn't support Vec<Template>,
// and elefren doesn't support anything but TOML
// TODO: I could technically just serialize the credentials to bincode as well
extern crate bincode;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;
extern crate rand;

mod madlibs;

use elefren::{Mastodon, MastodonClient, Registration, StatusBuilder};
use elefren::helpers::cli;
use elefren::helpers;
use elefren::entities::*;

use std::fs::File;
use rand::Rng;

#[derive(Deserialize, Serialize)]
struct BotStatus {
    last_noti_date: chrono::DateTime<chrono::Utc>,
    known_templates: Vec<madlibs::Template>,
}

fn solve_and_post(mastodon: Mastodon, mut template: &mut madlibs::Template, acct: Option<String>) {
    let home = mastodon.get_home_timeline().expect("couldn't fetch home timeline");
    for status in home.items_iter() {
        if status.account.acct == "madlibs" || status.content.contains("madlibs") {
            continue;
        }
        match madlibs::reduce_template(&mut template, status.content) {
            Some(mut text) => {
                let end = match acct {
                    Some(acct) => format!("cc @{}", acct),
                    None => String::new()
                };
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

fn process_mention(
        mastodon: Mastodon,
        notification: notification::Notification,
        add_template_to: &mut Vec<madlibs::Template>) {
    info!("mention from {}", notification.account.acct);
    let status = notification.status.unwrap();
    let text = status.content;
    let mut template = madlibs::to_template(text);
    add_template_to.push(template.to_vec());
    solve_and_post(mastodon, &mut template, Some(notification.account.acct));
}

fn post_random_madlib(mastodon: Mastodon, templates: &Vec<madlibs::Template>) {
    info!("posting random template");
    let mut template = rand::thread_rng().choose(templates).unwrap().clone();
    solve_and_post(mastodon, &mut template, None);
}

fn process_follow(mastodon: Mastodon, account: account::Account) {
    info!("followed by {}", account.acct);
    let result = mastodon
        .follow(account.id.parse().expect("id is invalid"));
    match result {
        Ok(_) => (),
        Err(err) => error!("{}", err),
    };
}

fn sleep(secs: u64) {
    std::thread::sleep(std::time::Duration::from_secs(secs));
}

fn poll_loop(mastodon: Mastodon) {
    let sleep_time = 60; // in seconds

    let mut bot_status = match File::open("status.bincode") {
        Ok(mut file) => {
            bincode::deserialize_from(file).unwrap()
        },
        Err(_) => BotStatus {
            // WARNING: Don't use this bot in the past
            last_noti_date: chrono::DateTime::from_utc(
                                chrono::naive::NaiveDateTime::from_timestamp(0, 0),
                                chrono::Utc),
            known_templates: vec![],
        }
    };

    let mut next_random = chrono::DateTime::from_utc(
                                chrono::naive::NaiveDateTime::from_timestamp(0, 0),
                                chrono::Utc);
    let mut rng = rand::thread_rng();
    loop {
        let notis = mastodon.notifications().expect("couldn't fetch notis");
        let now = chrono::Utc::now();
        if now >= next_random {
            let next_hours = rng.gen_range(1, 24);
            next_random = now + chrono::Duration::hours(next_hours);
            post_random_madlib(mastodon.clone(), &bot_status.known_templates);
        }
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
                notification::NotificationType::Mention => process_mention(mastodon.clone(), noti, &mut bot_status.known_templates),
                notification::NotificationType::Follow => process_follow(mastodon.clone(), noti.account),
                _ => (),
            };
        }
        // Now that we've finished our search, we can update our bot status
        bot_status.last_noti_date = last_noti_date_temp;
        // Serialize the bot status occasionally
        match File::create("status.bincode") {
            Ok(file) => {
                bincode::serialize_into(file, &bot_status)
                    .expect("couldn't serialize to file")
            },
            Err(_) => panic!("couldn't create/open status file")
        };
        sleep(sleep_time);
    }
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
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


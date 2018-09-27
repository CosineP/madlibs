// Mostly straight outta https://github.com/Aaronepower/Mammut README

extern crate elefren;
// extern crate toml;

use elefren::{Mastodon, MastodonClient, Registration, StatusBuilder};
use elefren::helpers::cli;
use elefren::helpers::toml;
use elefren::entities::*;

mod madlibs;

// struct BotStatus {
//     last_noti_date: DateTime<Utc>,
//     // known_templates: Vec<Template>, // TODO, when we start auto-posting
// };

fn process_mention(mastodon: Mastodon, notification: notification::Notification) {
    let status = notification.status.unwrap();
    let text = status.content;
    let mut template = madlibs::to_template(text);
    let home = mastodon.get_home_timeline().expect("couldn't fetch home timeline");
    for status in home.items_iter() {
        match madlibs::reduce_template(&mut template, status.content) {
            Some(mut text) => {
                let end = format!("\n\ncc @{}", notification.account.acct);
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

fn main() {
    let mastodon = match toml::from_file("credentials.toml") {
        Ok(data) => {
            Mastodon::from(data)
        }
        Err(_) => register(),
    };

    // let last_status = match toml::from_file("status.toml");
    // let last

    let notis = mastodon.notifications().expect("couldn't fetch notis");
    for noti in notis.initial_items {
        // if noti.created_at = 
        match noti.notification_type {
            notification::NotificationType::Mention => process_mention(mastodon.clone(), noti),
            notification::NotificationType::Follow => process_follow(mastodon.clone(), noti.account),
            _ => (),
        };
    }
}

fn register() -> Mastodon {
    let registration = Registration::new("https://beeping.town")
        .client_name("madlibs-bot")
        .build()
        .unwrap();
    let mastodon = cli::authenticate(registration).unwrap();

    toml::to_file(&*mastodon, "credentials.toml")
        .expect("could not save credentials");

    mastodon
}


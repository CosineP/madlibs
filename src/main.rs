// Mostly straight outta https://github.com/Aaronepower/Mammut README

extern crate elefren;

use std::fs::File;

use elefren::{Data, Mastodon, MastodonClient, Registration};
use elefren::helpers::cli;
use elefren::helpers::toml;
use elefren::entities::*;

mod madlibs;

// Modifies template in-line
// Returns whether the template has been fully reduced
fn reduce_template(template: &madlibs::Template, status: status::Status) -> bool {
    true
}

fn process_mention(mastodon: Mastodon, mention: status::Status) {
    let text = mention.content;
    let mut template = madlibs::to_template(text);
    let home = mastodon.get_home_timeline().expect("couldn't fetch home timeline");
    for status in home.items_iter() {
        if reduce_template(&template, status) {
            break;
        }
    }
    println!("{}", "doot");
}

fn process_follow(mastodon: Mastodon, account: account::Account) {
    mastodon.follow(account.id.parse().expect("id is invalid"));
}

fn main() {
    let mastodon = match toml::from_file("credentials.toml") {
        Ok(data) => {
            Mastodon::from(data)
        }
        Err(_) => register(),
    };

    let notis = mastodon.notifications().expect("couldn't fetch notis");
    for noti in notis.initial_items {
        match noti.notification_type {
            notification::NotificationType::Mention => process_mention(mastodon.clone(), noti.status.unwrap()),
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

    toml::to_file(&*mastodon, "credentials.toml");

    mastodon
}


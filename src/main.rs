// Mostly straight outta https://github.com/Aaronepower/Mammut README

extern crate mammut;
extern crate toml;

use std::io;
use std::fs::File;
use std::io::prelude::*;

use mammut::{Data, Mastodon, Registration, StatusBuilder};
use mammut::apps::{AppBuilder, Scopes};
use mammut::entities::*;

mod madlibs;

fn process_mention(mention: status::Status) {
    let text = mention.content;
    let template = madlibs::to_template(text);
    println!("{}", "doot");
}

fn process_follow(mastodon: Mastodon, account: account::Account) {
    mastodon.follow(account.id.parse().expect("id is invalid"));
}

fn main() {
    let mastodon = match File::open("credentials.toml") {
        Ok(mut file) => {
            let mut config = String::new();
            file.read_to_string(&mut config).unwrap();
            let data: Data = toml::from_str(&config).unwrap();
            Mastodon::from_data(data)
        },
        Err(_) => register(),
    };

    let notis = mastodon.notifications().expect("couldn't fetch notis");
    for noti in notis.initial_items {
        match noti.notification_type {
            notification::NotificationType::Mention => process_mention(noti.status.unwrap()),
            notification::NotificationType::Follow => process_follow(mastodon.clone(), noti.account),
            _ => (),
        };
    }
}

fn register() -> Mastodon {
    let app = AppBuilder {
        client_name: "madlibs-bot",
        redirect_uris: "urn:ietf:wg:oauth:2.0:oob",
        scopes: Scopes::All,
        website: None,
    };

    let mut registration = Registration::new("https://beeping.town");
    registration.register(app).unwrap();
    let url = registration.authorise().unwrap();

    println!("Click this link to authorise on Mastodon: {}", url);
    println!("Paste the returned authorisation code: ");

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let code = input.trim();
    let mastodon = registration.create_access_token(code.to_string()).unwrap();

    // Save app data for using on the next run
    let toml = toml::to_string(&*mastodon).unwrap();
    let mut file = File::create("credentials.toml").unwrap();
    file.write_all(toml.as_bytes()).unwrap();

    mastodon
}


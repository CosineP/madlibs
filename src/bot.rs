// Deals with the botty aspects: polling, sending, etc

use elefren::{Mastodon, MastodonClient, Registration, StatusBuilder, entities::status::Status};
use elefren::helpers::cli;
use elefren::helpers;
use elefren::entities::*;
use rand::Rng;

use std::fs::File;
use std::collections::HashSet;
use std::collections::HashMap;

use collection;
use pos;

use template::Template;
use collection::CollectionStatus;
use AccountID;

// elefren continues to use String in future versions so this is future-aware
type StatusID = String;

#[derive(Deserialize, Serialize)]
pub struct BotStatus {
    pub last_noti_date: chrono::DateTime<chrono::Utc>,
    pub known_templates: Vec<Template>,
    pub collection_toots: HashMap<StatusID, CollectionStatus>,
}

type BotError = elefren::errors::Error;
type Result<T> = std::result::Result<T, BotError>;

fn solve_and_post(mastodon: &Mastodon, template: &mut Template, used_statuses: &mut HashSet<String>, acct: Option<String>) -> Result<()> {
    let home = mastodon.get_home_timeline()?;
    for status in home.items_iter() {
        if status.account.acct == "madlibs"
            || status.content.contains("madlibs")
            || used_statuses.contains(&status.id) {
            continue
        }
        used_statuses.insert(status.id);
        match template.reduce(&status.content) {
            Some(mut text) => {
                let end = match acct {
                    Some(acct) => format!("cc @{}", acct),
                    None => String::new()
                };
                text.push_str(&end);
                mastodon.new_status(StatusBuilder {
                    status: text,
                    ..Default::default()
                })?;
                break
            },
            None => ()
        };
    }
    Ok(())
}

fn post_collection(mastodon: &Mastodon, template: &Template, acct: Option<AccountID>) -> Result<StatusID> {
    let mut text = String::from("let's play madlibs! this one's called\n");
    let title = match &template.title {
        Some(title) => title,
        None => "Untitled",
    };
    text.push_str(title);
    text.push('\n');
    for _ in 0..title.len() {
        text.push('=');
    }
    text.push_str("\ni need the following words:\n");
    for (pos, count) in template.requirements() {
        text.push_str(&format!("{}x: ", count));
        text.push_str(pos::pos_to_str(&pos));
    }
    if let Some(acct) = acct {
        text.push_str(&format!(
            "contribute one or more words by replying like this:
noun: hegemony
verbs: ruins
and comment on a separate line

cc @{}", acct));
    }
    Ok(mastodon.new_status(StatusBuilder {
        status: text,
        ..Default::default()
    })?.id)
}

fn process_template_mention(mastodon: &Mastodon, notification: notification::Notification, bot_status: &mut BotStatus, used_statuses: &mut HashSet<String>) -> Result<()> {
    let status = notification.status.unwrap();
    let acct = notification.account.acct;
    let mut template = match Template::parse(&status.content) {
        Ok(plate) => plate,
        Err(e) => {
            toot_parse_error(mastodon, &status, e, "template")?;
            return Ok(());
        }
    };
    // Ignore mentions that don't include any template words
    if template.body.len() > 1 {
        if template.title.is_some() {
            let toot_id = post_collection(mastodon, &template, Some(acct))?;
            // hasn't been inserted yet so no -1
            let plate_id = bot_status.known_templates.len();
            bot_status.collection_toots.insert(toot_id, CollectionStatus::new(plate_id));
        } else {
            solve_and_post(mastodon, &mut template, used_statuses, Some(acct))?;
        }
        bot_status.known_templates.push(template);
    }
    Ok(())
}

fn toot_parse_error<E: std::fmt::Display>(mastodon: &Mastodon, status: &Status, e: E, kind: &str) -> Result<()> {
    // another elefren annoyance CHECK
    let in_reply_to_id = match status.id.parse() {
        Ok(o) => Some(o),
        Err(e) => {
            warn!("foreign string id didn't parse to native u64 id: {}", e);
            None
        }
    };
    mastodon.new_status(StatusBuilder {
        status: format!("@{} could not parse your {}: {}", status.account.acct, kind, e),
        visibility: Some(status.visibility),
        in_reply_to_id,
        ..Default::default()
    })?;
    Ok(())
}

// returns true if this WAS a valid, live collection mention, false if it wasn't
fn process_collection_mention(mastodon: &Mastodon, notification: &notification::Notification, bot_status: &mut BotStatus) -> Result<bool> {
    let status = notification.status.as_ref().unwrap();
    // we can't chain if-let, (feature(let_chains) doesn't even work),
    // returns will do the trick
    if let Some(reply_id) = &status.in_reply_to_id {
        if let Some(collection) = bot_status.collection_toots.get_mut(reply_id) {
            let status = notification.status.as_ref().unwrap();
            let resp = match collection::parse_response(&status.content) {
                Ok(resp) => resp,
                Err(e) => {
                    toot_parse_error(mastodon, status, e, "response")?;
                    return Ok(true);
                }
            };
            collection.add_responses(resp);
            match collection.check_done(&bot_status.known_templates) {
                Some(mut text) => {
                    text.push_str(&collection.get_participant_ats());
                    mastodon.new_status(StatusBuilder {
                        status: text,
                        ..Default::default()
                    })?;
                }
                // still waiting around
                None => (),
            }
            return Ok(true)
        }
    }
    Ok(false)
}

fn process_mention(
        mastodon: &Mastodon,
        notification: notification::Notification,
        bot_status: &mut BotStatus,
        used_statuses: &mut HashSet<String>) -> Result<()> {
    info!("mention from {}", &notification.account.acct);
    if !process_collection_mention(mastodon, &notification, bot_status)? {
        process_template_mention(mastodon, notification, bot_status, used_statuses)?;
    }
    Ok(())
}

fn post_random_madlib(mastodon: &Mastodon, templates: &Vec<Template>, used_statuses: &mut HashSet<String>) -> Result<()> {
    info!("posting random template");
    // Solve and post changes the template which we don't want, so we clone
    let mut template = rand::thread_rng().choose(templates).unwrap().clone();
    solve_and_post(mastodon, &mut template, used_statuses, None)?;
    Ok(())
}

fn process_follow(mastodon: &Mastodon, account: account::Account) -> Result<()> {
    info!("followed by {}", account.acct);
    match mastodon.follow(account.id.parse().unwrap()) {
        Ok(_) => (),
        // TODO: i suppose we now have a reason to eventually update elefren
        Err(e) => warn!("recieved error code after following account {}: {}; doing nothing (usually fine; elefren bug?)", account.acct, e),
    }
    Ok(())
}

fn sleep(secs: u64) {
    std::thread::sleep(std::time::Duration::from_secs(secs));
}

fn get_status() -> BotStatus {
    match File::open("status.bincode") {
        Ok(file) => {
            bincode::deserialize_from(file).unwrap()
        },
        Err(_) => BotStatus {
            // WARNING: Don't use this bot in the past
            last_noti_date: chrono::DateTime::from_utc(
                                chrono::naive::NaiveDateTime::from_timestamp(0, 0),
                                chrono::Utc),
            known_templates: vec![],
            collection_toots: HashMap::new(),
        }
    }
}

fn sync_exp_backoff<F, T>(mut call: F) where
    F: FnMut() -> Result<T> {
    let mut time = 1;
    loop {
        match call() {
            Ok(_) => return,
            Err(e) => {
                error!("{}, trying again exp {}", e, time);
                // More than two hours waiting = 4 hours total = give up
                const CUTOFF: u64 = 60 * 60 * 2;
                if time > CUTOFF {
                    error!("giving up on exponential backoff");
                    return;
                }
                sleep(time);
                time *= 2;
            }
        }
    }
}

fn poll_notis(mastodon: &Mastodon, bot_status: &mut BotStatus, used_statuses: &mut HashSet<String>) -> Result<()> {
    let mut last_noti_date_temp = bot_status.last_noti_date;
    let notis = mastodon.notifications()?;
    for noti in notis.initial_items {
        // If we have caught up with ourselves
        if noti.created_at <= bot_status.last_noti_date {
            // Exit, the loop is done, persistence is done outside of loop
            break;
        }
        // Only if we're on first run, getting the most recent noti
        // otherwise our check if we've caught up would say "yes"
        if noti.created_at > last_noti_date_temp {
            last_noti_date_temp = noti.created_at;
        }

        match noti.notification_type {
            notification::NotificationType::Mention => process_mention(&mastodon, noti, bot_status, used_statuses)?,
            notification::NotificationType::Follow => process_follow(&mastodon, noti.account)?,
            _ => (),
        }
    }
    bot_status.last_noti_date = last_noti_date_temp;
    Ok(())
}

fn poll_loop(mastodon: &Mastodon) {
    let sleep_time = 60; // in seconds

    let mut bot_status = get_status();
    let mut used_statuses = HashSet::new();

    let mut next_random = chrono::DateTime::from_utc(
                                chrono::naive::NaiveDateTime::from_timestamp(0, 0),
                                chrono::Utc);
    let mut rng = rand::thread_rng();
    let mut first_time = true;
    loop {
        let now = chrono::Utc::now();
        if now >= next_random {
            if !first_time {
                sync_exp_backoff(|| post_random_madlib(&mastodon, &bot_status.known_templates, &mut used_statuses));
            }
            let next_hours = rng.gen_range(1, 24);
            next_random = now + chrono::Duration::hours(next_hours);
            first_time = false;
        }
        sync_exp_backoff(|| poll_notis(mastodon, &mut bot_status, &mut used_statuses));
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

pub fn run() {
    let mastodon = match helpers::toml::from_file("credentials.toml") {
        Ok(data) => {
            Mastodon::from(data)
        }
        Err(_) => register(),
    };
    poll_loop(&mastodon)
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


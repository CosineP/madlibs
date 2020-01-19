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

const MAX_STATUS_LENGTH: usize = 512;
const MAX_TEMPLATE_LENGTH: usize = 4096;

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
            Some(text) => {
                let end = match acct {
                    Some(acct) => format!("cc @{}", acct),
                    None => String::new()
                };
                post_vec(mastodon, &split(&text, &end), None)?;
                break
            },
            None => ()
        };
    }
    Ok(())
}

fn format_collection_toot(template: &Template, acct: Option<AccountID>) -> String {
    let title = match &template.title {
        Some(title) => title,
        None => "Untitled",
    };
    let mut text = format!("let's play madlibs! this one's called: **{}**

i need the following words:
", title);
    for (pos, count) in template.requirements() {
        text.push_str(&format!("\n{}x: {}", count, pos::pos_to_str(&pos)));
    }
    text.push_str("

contribute one or more words by replying like this:
noun: hegemony
verbs: sucks");
    if let Some(acct) = acct {
        text.push_str(&format!("\n\ncc @{}", acct));
    }
    text
}

fn post_collection(mastodon: &Mastodon, template: &Template, acct: Option<AccountID>) -> Result<StatusID> {
    Ok(mastodon.new_status(StatusBuilder::new()
        .status(format_collection_toot(template, acct))
        .build()?
    )?.id)
}

fn process_template_mention(mastodon: &Mastodon, notification: notification::Notification, bot_status: &mut BotStatus, used_statuses: &mut HashSet<String>) -> Result<()> {
    info!("... it was a non-collection mention");
    let status = notification.status.unwrap();
    let acct = notification.account.acct;
    if status.content.len() > MAX_TEMPLATE_LENGTH {
        mastodon.new_status(StatusBuilder::new()
            .status(&format!("templates have a max length of {}", MAX_STATUS_LENGTH))
            .visibility(status.visibility)
            .in_reply_to(&status.id)
            .build()?
        )?;
    }
    let mut template = match Template::parse(&status.content) {
        Ok(plate) => plate,
        Err(e) => {
            toot_parse_error(mastodon, &status, e, "template")?;
            return Ok(());
        }
    };
    // Ignore mentions that don't include any template words
    if template.body.len() > 1 {
        info!("... with a valid template");
        if template.title.is_some() {
            info!("... and a title (manual mode)");
            let toot_id = post_collection(mastodon, &template, Some(acct.clone()))?;
            // hasn't been inserted yet so no -1
            let plate_id = bot_status.known_templates.len();
            bot_status.collection_toots.insert(toot_id, CollectionStatus::new(plate_id, acct));
        } else {
            solve_and_post(mastodon, &mut template, used_statuses, Some(acct))?;
        }
        bot_status.known_templates.push(template);
    }
    Ok(())
}

fn toot_parse_error<E: std::fmt::Display>(mastodon: &Mastodon, status: &Status, e: E, kind: &str) -> Result<()> {
    mastodon.new_status(StatusBuilder::new()
        .status(format!("@{} could not parse your {}: {}", status.account.acct, kind, e))
        .visibility(status.visibility)
        .in_reply_to(&status.id)
        .build()?
    )?;
    Ok(())
}

// CHECK: they could be COWs but honestly i don't care enough
fn split(mut text: &str, append: &str) -> Vec<String> {
    // we don't have to worry about 10/10 because MAX_TEMPLATE_LENGTH
    debug_assert!(MAX_TEMPLATE_LENGTH < MAX_STATUS_LENGTH * 9,
        "need to revamp OUT_OF_CHARS");
    const N_OF_N_LEN: usize = 6; // \s(n/n)
    const MAX_PART_STATUS_LENGTH: usize = MAX_STATUS_LENGTH - N_OF_N_LEN;
    let total = text.len() + append.len();
    // if just one, exit early with special logic
    // not redundant with max because of (n/n) markers
    if total <= MAX_STATUS_LENGTH {
        return vec![format!("{}{}", text, append)];
    }
    // round up
    let max = (text.len() - 1) / MAX_PART_STATUS_LENGTH + 1;
    let mut posts = vec![];
    for curr in 0..max {
        let mut boundary = MAX_PART_STATUS_LENGTH;
        while !text.is_char_boundary(boundary) {
            boundary -= 1;
        }
        let (chunk, rest) = text.split_at(boundary);
        text = rest;
        // TODO: technically this gives unexpected behavior if text is 512,
        // append is >0, so it's not "just one", so we end up splitting it over
        // two it'll still say (1/1)
        //
        // should display as ordinal so +1
        let all = format!("{} ({}/{})", chunk, curr + 1, max);
        // last one special handling
        if curr + 1 == max {
            if chunk.len() + append.len() > MAX_STATUS_LENGTH {
                // otherwise our math is wrong
                assert!(all.len() < MAX_STATUS_LENGTH);
                // redundant with other format but blech
                posts.push(all);
                posts.push(append.to_string());
            } else {
                posts.push(format!("{}{}", all, append));
            }
        } else {
            posts.push(all);
        }
    }
    // make sure we munched it all
    assert_eq!(text, "");
    posts
}

fn post_vec(mastodon: &Mastodon, posts: &[String], mut reply_id: Option<String>) -> Result<()> {
    for post in posts {
        let mut partial = StatusBuilder::new();
        let status = if let Some(id) = reply_id {
            partial.in_reply_to(id)
        } else { &mut partial }
            .status(post)
            .build()?;
        reply_id = Some(mastodon.new_status(status)?.id);
    }
    Ok(())
}

// returns true if this WAS a valid, live collection mention, false if it wasn't
fn process_collection_mention(mastodon: &Mastodon, notification: &notification::Notification, bot_status: &mut BotStatus) -> Result<bool> {
    let status = notification.status.as_ref().unwrap();
    // we can't chain if-let, (feature(let_chains) doesn't even work),
    // returns will do the trick
    if let Some(reply_id) = &status.in_reply_to_id {
        if let Some(collection) = bot_status.collection_toots.get_mut(reply_id) {
            info!("... it was a collection mention");
            let status = notification.status.as_ref().unwrap();
            let resp = match collection::parse_response(&status.content) {
                Ok(resp) => resp,
                Err(e) => {
                    toot_parse_error(mastodon, status, e, "response")?;
                    return Ok(true);
                }
            };
            collection.add_responses(resp);
            collection.add_participant(notification.account.acct.clone());
            match collection.check_done(&bot_status.known_templates) {
                Some(text) => {
                    let ccs = collection.get_participant_ats();
                    post_vec(mastodon, &split(&text, &ccs), Some(reply_id.clone()))?;
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

fn process_follow(mastodon: &Mastodon, account: &account::Account) -> Result<()> {
    info!("followed by {}", &account.acct);
    mastodon.follow(&account.id).and(Ok(()))
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
            notification::NotificationType::Follow => process_follow(&mastodon, &noti.account)?,
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

#[cfg(test)]
mod test {
    use super::{split, MAX_STATUS_LENGTH};
    use std::iter::repeat;
    #[test]
    fn test_split() {
        let one_toot: String = repeat('.').take(MAX_STATUS_LENGTH).collect();
        assert_eq!(split(&one_toot, ""), vec![one_toot]);
        // - 20 for (n/n) w/ lenience
        let long_toot: String = repeat('.').take(MAX_STATUS_LENGTH * 2 - 20).collect();
        // rather than do a bunch of error prone munging, we spot check a few things
        let got = split(&long_toot, "");
        println!("{:?}", got);
        assert_eq!(got.len(), 2);
        assert!(got[0].ends_with(" (1/2)"));
        // this minus ten is to account for a KNOWN BUG that needs to be fixed
        // (TODO) around erronious (1/1)
        let for_append: String = repeat('.').take(MAX_STATUS_LENGTH - 10).collect();
        let append = "this would put over for sure for sure";
        let got = split(&for_append, append);
        println!("{:?}", got);
        assert_eq!(got.len(), 2);
        assert_eq!(got[1], append);
    }
}


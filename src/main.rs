// Deals with the botty aspects: polling, sending, etc

extern crate madlibs;

use madlibs::bot;

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    bot::run();
}


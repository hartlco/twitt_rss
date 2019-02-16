extern crate egg_mode;
extern crate tokio_core;
extern crate rss;
extern crate rouille;
extern crate config;

use rouille::Response;
use tokio_core::reactor::Core;
use rss::ChannelBuilder;
use rss::ItemBuilder;
use egg_mode::tweet::Tweet;
use std::env;

fn main() {
    let port = config_value("port");
    println!("Starting server on port {}", port);
    rouille::start_server(format!("0.0.0.0:{}", port), move |_request| {
        println!("{}", "Getting request");
        return Response::from_data("application/rss+xml", feed());
    });
}

fn feed() -> String {
    let consumer_key = config_value("consumer_key");
    let consumer_secret = config_value("consumer_secret");

    let access_token = config_value("access_token");
    let access_token_secret = config_value("access_token_secret");


    let con_token = egg_mode::KeyPair::new(consumer_key, consumer_secret);
    let access_token = egg_mode::KeyPair::new(access_token, access_token_secret);
    let token = egg_mode::Token::Access {
        consumer: con_token,
        access: access_token,
    };

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let rustlang = core.run(egg_mode::user::show(&config_value("username"), &token, &handle)).unwrap();
    let lists = core.run(egg_mode::list::list(rustlang.id, true, &token, &handle)).unwrap();

    for list in lists {
        if list.name == config_value("listname") {
            let listid = egg_mode::list::ListID::from_id(list.id);
            let timeline = egg_mode::list::statuses(listid, true, &token, &handle).with_page_size(100);
            let tweets = core.run(timeline.start()).unwrap();
            return create_feed(tweets.1);
        }
    }

    return "".to_string();
}

fn create_feed(tweets: egg_mode::Response<std::vec::Vec<Tweet>>) -> String {
    let mut tweet_items = Vec::new();

    for tweet in tweets {
        let mut guid = rss::Guid::default();
        guid.set_value(tweet.id.to_string());
        guid.set_permalink(false);

        let username = username_for(&tweet);

        let pub_date = tweet.created_at.to_rfc2822();
        
        let item =  ItemBuilder::default()
        .description(content_for(&tweet))
        .title(username.to_string())
        .pub_date(pub_date)
        .link(format!("https://twitter.com/{}/statuses/{}", username, tweet.id.to_string()))
        .guid(guid)
        .build()
        .unwrap();

        tweet_items.push(item);
    }

    let channel = ChannelBuilder::default()
    .title(config_value("rss_title"))
    .items(tweet_items)
    .link(config_value("rss_url"))
    .description(config_value("rss_description"))
    .build()
    .unwrap();

    channel.write_to(::std::io::sink()).unwrap();
    let string = channel.to_string();
    return string;
}

fn content_for(tweet: &Tweet) -> String {
    let mut content = format!("<p>{}</p>", replaced_content_for(tweet)).to_string();

    if let Some(quote) = &tweet.quoted_status {
        content = format!("{}\n{}:\n<blockquote>{}</blockquote>", content, username_for(&quote), replaced_content_for(quote));
    }

    if let Some(retweet) = &tweet.retweeted_status {
        content = format!("<p>Retweet {}: {}</p>", username_for(&retweet), replaced_content_for(retweet)).to_string();
    }

    return content;
}

fn username_for(tweet: &Tweet) -> String {
    if let Some(user) = &tweet.user {
        return user.name.to_string();
    } else {
        return "No username".to_string();
    }
}

fn replaced_content_for(tweet: &Tweet) -> String {
    let mut content = tweet.text.to_string();

    for url in &tweet.entities.urls {
        let html_url = format!("<a href=\"{}\">{}</a>", url.expanded_url, url.display_url);
        content = content.replace(&url.url, &html_url);
    }

    if let Some(entities) = &tweet.extended_entities {
        for media in &entities.media {
            content = format!("\n{}<img src=\"{}\">", content, media.media_url_https);
            content = content.replace(&media.url, "")
        }
    }

    return content;
}

fn config_value(key: &str) -> String {
    let args: Vec<String> = env::args().collect();
    let config_name = &args[1];

    let mut settings = config::Config::default();
    settings.merge(config::File::with_name(config_name)).unwrap();
    match settings.get_str(key) {
        Ok(value) => {
            return value
        }

        Err(_e) => {
            panic!(format!("Invalid key {}", key));
        }
    }
}
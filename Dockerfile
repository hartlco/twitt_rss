FROM rust:1.48

WORKDIR /usr/src/twittrss

COPY . .

RUN cargo install --path .

CMD ["twitt_rss", "Config.toml"]

FROM rust:1.81.0-bookworm

WORKDIR /usr/src/server
COPY ./src ./src
COPY ./Cargo.toml .
COPY ./Cargo.lock .

RUN cargo install --path "." --locked

CMD ["server", "0.0.0.0"]
EXPOSE 8080
EXPOSE 8081

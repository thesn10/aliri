use std::time::Duration;

use aliri::jwt;
use aliri_clock::DurationSecs;
use aliri_tokens::{
    backoff, jitter, sources, ClientId, ClientSecret, TokenLifetimeConfig, TokenRefresher,
    TokenStatus,
};
use clap::Parser;
use tokio::time;

#[derive(Debug, Parser)]
struct Opts {
    /// The issuing authority's token request URL
    #[clap(short, long, env)]
    token_url: reqwest::Url,

    /// The client ID of the client
    #[clap(short, long, env)]
    client_id: ClientId,

    /// The client secret used to identify the client to the issuing authority
    #[clap(short = 's', long, env, hide_env_values = true)]
    client_secret: ClientSecret,

    /// The audience to request a token for
    #[clap(short, long, env)]
    audience: jwt::Audience,

    /// The local file used to cache credentials
    #[clap(
        short = 'f',
        long,
        env,
        name = "FILE",
        default_value = ".credentials.json"
    )]
    credentials_file: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let opts = Opts::parse();

    let client = reqwest::Client::builder().https_only(true).build()?;

    let credentials = sources::oauth2::dto::ClientCredentialsWithAudience {
        credentials: sources::oauth2::dto::ClientCredentials {
            client_id: opts.client_id,
            client_secret: opts.client_secret,
        }
        .into(),
        audience: opts.audience,
    };

    let fallback = sources::oauth2::ClientCredentialsTokenSource::new(
        client,
        opts.token_url,
        credentials,
        TokenLifetimeConfig::default(),
    );

    let file_source = sources::file::FileTokenSource::new(opts.credentials_file);

    let token_source =
        sources::cache::CachedTokenSource::new(fallback).with_cache("file", file_source);

    let jitter_source = jitter::RandomEarlyJitter::new(DurationSecs(60));

    let refresher = TokenRefresher::spawn_from_token_source(
        token_source,
        jitter_source,
        backoff::ErrorBackoffConfig::default(),
    )
    .await?;

    let watcher = refresher.get_watcher();

    tracing::info!(
        token = format_args!("{:#?}", watcher.token().access_token()),
        "first access token"
    );

    tokio::spawn(watcher.clone().watch(|token| async move {
        tracing::info!(
            token = format_args!("{:#?}", token.access_token()),
            "new access token"
        );
    }));

    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        let token = watcher.token();
        let status = token.token_status();
        match status {
            TokenStatus::Fresh => {
                tracing::debug!(
                    ?status,
                    stale = token.stale().0,
                    expiry = token.expiry().0,
                    "pulled token"
                )
            }
            TokenStatus::Stale => {
                tracing::warn!(
                    ?status,
                    stale = token.stale().0,
                    expiry = token.expiry().0,
                    "pulled token"
                )
            }
            TokenStatus::Expired => {
                tracing::error!(
                    ?status,
                    stale = token.stale().0,
                    expiry = token.expiry().0,
                    "pulled token"
                )
            }
        }
    }
}

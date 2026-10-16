//! `uc token`: create, list and delete personal access tokens.

use clap::Subcommand;
use uc_api::CreatePat;
use uc_client::Page;

use super::options::help;
use super::{Context, Result, upstream};

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum TokenOp {
    /// Create a personal access token
    Create {
        #[arg(long, help = help::NAME)]
        name: String,
        /// When the token expires, in epoch milliseconds
        #[arg(long)]
        expires_at: Option<i64>,
    },
    /// List your personal access tokens
    List,
    /// Delete a personal access token
    Delete {
        /// The id of the token
        #[arg(long)]
        id: String,
    },
}

pub async fn run(c: &Context<'_>, op: TokenOp) -> Result<String> {
    match op {
        TokenOp::Create { name, expires_at } => {
            let token = c
                .client
                .create_pat(&CreatePat { name, expires_at })
                .await
                .map_err(upstream("createToken"))?;
            Ok(c.json(&serde_json::to_value(&token)?))
        }
        TokenOp::List => {
            let list = c
                .client
                .list_pats(&Page::default())
                .await
                .map_err(upstream("listTokens"))?;
            Ok(c.json(&serde_json::to_value(&list.tokens)?))
        }
        TokenOp::Delete { id } => {
            c.client
                .revoke_pat(&id)
                .await
                .map_err(upstream("deleteToken"))?;
            Ok(String::new())
        }
    }
}

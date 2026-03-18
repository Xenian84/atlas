//! Atlas CLI — inspect and manage the Atlas platform from the command line.
//!
//! Usage:
//!   atlas status                     — full system health check
//!   atlas pulse                      — network pulse snapshot
//!   atlas tx <sig>                   — look up a transaction
//!   atlas wallet <address>           — unified wallet overview
//!   atlas token <mint> [--holders]   — token info + optional top holders
//!   atlas block <slot>               — block overview
//!   atlas stream [--count N]         — tail live shred stream
//!   atlas keygen                     — generate an X1 keypair
//!   atlas rpc                        — print RPC endpoint info
//!   atlas usage [key-prefix]         — API key usage stats
//!   atlas keys list                  — list API keys (admin)
//!   atlas keys create <name>         — create an API key (admin)
//!   atlas keys revoke <id>           — revoke an API key (admin)
//!
//!   Add --json to any command for machine-readable output.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod cmd;

#[derive(Parser)]
#[command(
    name    = "atlas",
    about   = "Atlas platform CLI — X1 blockchain data layer",
    version = "0.1.0",
)]
pub struct Cli {
    /// Atlas API base URL
    #[arg(long, env = "ATLAS_API_URL", default_value = "http://localhost:8888")]
    pub api: String,

    /// Redis URL for stream commands
    #[arg(long, env = "ATLAS_REDIS_URL", default_value = "redis://127.0.0.1:6379")]
    pub redis: String,

    /// X1 RPC URL — used as on-chain fallback
    #[arg(long, env = "ATLAS_RPC_URL", default_value = "http://localhost:8899")]
    pub rpc: String,

    /// API key for authenticated endpoints
    #[arg(long, env = "ATLAS_API_KEY", default_value = "atlas-admin-key-change-in-production")]
    pub key: String,

    /// Output raw JSON (ideal for scripts and AI agents)
    #[arg(long, short = 'j', global = true)]
    pub json: bool,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Full system health check (validator, services, streams, DB)
    Status,

    /// Network pulse — slot, TPS, indexer stats, top programs
    Pulse,

    /// Look up a transaction by signature
    Tx {
        /// Transaction signature (base58)
        sig: String,
    },

    /// Unified wallet overview — balance, tx history, tokens, identity
    Wallet {
        /// Wallet address (base58)
        address: String,
    },

    /// Token overview — metadata, supply, holders
    Token {
        /// Token mint address (base58)
        mint: String,
        /// Show top 10 holders
        #[arg(short = 'H', long)]
        holders: bool,
    },

    /// Block overview — tx count, fees, programs
    Block {
        /// Slot number
        slot: u64,
    },

    /// Tail live transaction events from the shred stream
    Stream {
        /// Number of recent events to show (default: 10)
        #[arg(short, long, default_value = "10")]
        count: usize,
        /// Watch continuously (Ctrl-C to stop)
        #[arg(short, long)]
        watch: bool,
    },

    /// Generate a new X1 keypair and print the address + secret key path
    Keygen {
        /// Output path for keypair JSON (default: ~/.atlas/keypair.json)
        #[arg(long)]
        output: Option<String>,
    },

    /// Print RPC and WebSocket endpoint URLs for this Atlas instance
    Rpc,

    /// Show API key usage statistics
    Usage {
        /// Filter by key prefix (optional — shows all keys if omitted)
        key_prefix: Option<String>,
    },

    /// API key management (admin only)
    Keys {
        #[command(subcommand)]
        action: KeysCmd,
    },
}

#[derive(Subcommand)]
enum KeysCmd {
    /// List all API keys
    List,
    /// Create a new API key
    Create {
        /// Key display name
        name: String,
        /// Tier (free|starter|pro|enterprise)
        #[arg(long, default_value = "free")]
        tier: String,
        /// Requests per minute limit
        #[arg(long, default_value = "300")]
        rpm: i32,
    },
    /// Revoke an API key by ID
    Revoke {
        /// Key UUID
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Cmd::Status  => cmd::status::run(&cli.api, &cli.redis, cli.json).await,
        Cmd::Pulse   => cmd::pulse::run(&cli.api, cli.json).await,
        Cmd::Tx { sig } => cmd::tx::run(&cli.api, &sig, cli.json).await,
        Cmd::Wallet { address } => cmd::wallet::run(&cli.api, &cli.rpc, &address, &cli.key, cli.json).await,
        Cmd::Token { mint, holders } => cmd::token::run(&cli.api, &mint, &cli.key, holders, cli.json).await,
        Cmd::Block { slot } => cmd::block::run(&cli.api, slot, &cli.key, cli.json).await,
        Cmd::Stream { count, watch } => cmd::stream::run(&cli.redis, count, watch).await,
        Cmd::Keygen { output } => cmd::keygen::run(output, cli.json).await,
        Cmd::Rpc => cmd::rpc::run(&cli.api, &cli.rpc, cli.json).await,
        Cmd::Usage { key_prefix } => cmd::usage::run(&cli.api, &cli.key, key_prefix.as_deref(), cli.json).await,
        Cmd::Keys { action } => match action {
            KeysCmd::List => cmd::keys::run_list(&cli.api, &cli.key, cli.json).await,
            KeysCmd::Create { name, tier, rpm } =>
                cmd::keys::run_create(&cli.api, &cli.key, &name, &tier, rpm, cli.json).await,
            KeysCmd::Revoke { id } =>
                cmd::keys::run_revoke(&cli.api, &cli.key, &id, cli.json).await,
        },
    }
}

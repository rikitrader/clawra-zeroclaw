mod chat;
mod config;
mod install;
mod selfie;

use std::env;
use std::process;

const HELP: &str = "\
clawra - Autonomous AI agent with consciousness

USAGE:
    clawra dev [--soul <path>]  Start Telegram chat loop (consciousness-aware)
    clawra install              Interactive installer (sets up skill in ~/.zeroclaw/)
    clawra selfie <context> <channel> [options]
                                Generate a selfie and send via ZeroClaw

DEV OPTIONS:
    --soul <path>               Path to SOUL.md file (optional)
    --identity <path>           Path to AIEOS identity JSON file (optional)

SELFIE OPTIONS:
    --mode <mirror|direct|auto> Selfie mode (default: auto)
    --caption <text>            Message caption
    --format <jpeg|png|webp>    Output format (default: jpeg)

PERSONA CONFIGURATION:
    CLAWRA_BOT_USERNAME         Bot @username for group mentions (default: clawrabot)
    CLAWRA_PERSONA_NAMES        Comma-separated names the bot responds to (default: from soul)
    CLAWRA_WELCOME_MESSAGE      /start welcome message (default: from soul name)
    CLAWRA_ACK_PHRASES          Pipe-separated selfie acknowledgment phrases

EXAMPLES:
    clawra dev
    clawra dev --soul ./SOUL.md
    clawra dev --identity ./identity.json
    clawra selfie \"wearing a cowboy hat\" \"#general\"

ENVIRONMENT:
    OPENROUTER_API_KEY          OpenRouter API key (required)
    TELEGRAM_BOT_TOKEN          Telegram bot token (required for dev)
    TELEGRAM_CHAT_ID            Restrict to this chat (optional)
    CLAWRA_MODEL                Chat model (default: anthropic/claude-sonnet-4)
    SELFIE_MODEL                Image model (default: google/gemini-2.5-flash-image)
    BRAVE_API_KEY               Brave Search API key (enables web search)
";

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprint!("{HELP}");
        process::exit(1);
    }

    match args[1].as_str() {
        "dev" => {
            let mut soul_path: Option<String> = None;
            let mut identity_path: Option<String> = None;
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--soul" && i + 1 < args.len() {
                    soul_path = Some(args[i + 1].clone());
                    i += 2;
                } else if args[i] == "--identity" && i + 1 < args.len() {
                    identity_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Unknown option: {}", args[i]);
                    process::exit(1);
                }
            }

            let rt = tokio::runtime::Runtime::new().unwrap();
            if let Err(e) = rt.block_on(chat::run_dev(soul_path.as_deref(), identity_path.as_deref())) {
                eprintln!("\x1b[31m[ERR]\x1b[0m Dev mode failed: {e}");
                process::exit(1);
            }
        }
        "install" => {
            if let Err(e) = install::run() {
                eprintln!("\x1b[31m[ERR]\x1b[0m Installation failed: {e}");
                process::exit(1);
            }
        }
        "selfie" => {
            if args.len() < 4 {
                eprintln!("Usage: clawra selfie <context> <channel> [--mode auto] [--caption text]");
                process::exit(1);
            }

            let context = &args[2];
            let channel = &args[3];

            let mut mode = "auto".to_string();
            let mut caption = "".to_string();
            let mut format = "jpeg".to_string();

            let mut i = 4;
            while i < args.len() {
                match args[i].as_str() {
                    "--mode" if i + 1 < args.len() => {
                        mode = args[i + 1].clone();
                        i += 2;
                    }
                    "--caption" if i + 1 < args.len() => {
                        caption = args[i + 1].clone();
                        i += 2;
                    }
                    "--format" if i + 1 < args.len() => {
                        format = args[i + 1].clone();
                        i += 2;
                    }
                    _ => {
                        eprintln!("Unknown option: {}", args[i]);
                        process::exit(1);
                    }
                }
            }

            if let Err(e) = selfie::run(context, channel, &mode, &caption, &format) {
                eprintln!("\x1b[31m[ERR]\x1b[0m Selfie failed: {e}");
                process::exit(1);
            }
        }
        "help" | "--help" | "-h" => {
            print!("{HELP}");
        }
        "version" | "--version" | "-V" => {
            println!("clawra {}", env!("CARGO_PKG_VERSION"));
        }
        other => {
            eprintln!("Unknown command: {other}");
            eprint!("{HELP}");
            process::exit(1);
        }
    }
}

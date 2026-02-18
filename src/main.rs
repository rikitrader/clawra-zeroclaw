mod config;
mod install;
mod selfie;

use std::env;
use std::process;

const HELP: &str = "\
clawra - Selfie superpowers for ZeroClaw agents

USAGE:
    clawra install              Interactive installer (sets up skill in ~/.zeroclaw/)
    clawra selfie <context> <channel> [options]
                                Generate a selfie and send via ZeroClaw

SELFIE OPTIONS:
    --mode <mirror|direct|auto> Selfie mode (default: auto)
    --caption <text>            Message caption
    --format <jpeg|png|webp>    Output format (default: jpeg)

EXAMPLES:
    clawra install
    clawra selfie \"wearing a cowboy hat\" \"#general\"
    clawra selfie \"a cozy cafe\" \"#photography\" --mode direct --caption \"Vibes!\"

ENVIRONMENT:
    FAL_KEY                     fal.ai API key (required for selfie)
    ZEROCLAW_GATEWAY_URL        Gateway URL (default: http://localhost:8080)
    ZEROCLAW_GATEWAY_TOKEN      Gateway paired bearer token
";

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprint!("{HELP}");
        process::exit(1);
    }

    match args[1].as_str() {
        "install" => {
            if let Err(e) = install::run() {
                eprintln!("\x1b[31m✗\x1b[0m Installation failed: {e}");
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

            // Parse optional flags
            let mut mode = "auto".to_string();
            let mut caption = "Edited with Grok Imagine".to_string();
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
                eprintln!("\x1b[31m✗\x1b[0m Selfie failed: {e}");
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

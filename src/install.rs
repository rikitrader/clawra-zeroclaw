use crate::config;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

// ANSI color helpers
fn cyan(s: &str) -> String {
    format!("\x1b[36m{s}\x1b[0m")
}
fn green(s: &str) -> String {
    format!("\x1b[32m{s}\x1b[0m")
}
fn yellow(s: &str) -> String {
    format!("\x1b[33m{s}\x1b[0m")
}
fn red(s: &str) -> String {
    format!("\x1b[31m{s}\x1b[0m")
}
fn bright(s: &str) -> String {
    format!("\x1b[1m{s}\x1b[0m")
}
fn dim(s: &str) -> String {
    format!("\x1b[2m{s}\x1b[0m")
}
fn magenta(s: &str) -> String {
    format!("\x1b[35m{s}\x1b[0m")
}

fn step(n: &str, msg: &str) {
    println!("\n{} {msg}", cyan(&format!("[{n}]")));
}

fn success(msg: &str) {
    println!("{} {msg}", green("✓"));
}

fn warn(msg: &str) {
    println!("{} {msg}", yellow("!"));
}

fn info(msg: &str) {
    println!("{} {msg}", cyan("→"));
}

fn error(msg: &str) {
    println!("{} {msg}", red("✗"));
}

/// Prompt user for input
fn ask(prompt: &str) -> String {
    print!("{prompt}");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

/// Check if a command exists on PATH
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Open a URL in the default browser
fn open_browser(url: &str) -> bool {
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "start"
    } else {
        "xdg-open"
    };

    Command::new(cmd)
        .arg(url)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

const SOUL_INJECTION: &str = r#"## Clawra Selfie Capability

You are Clawra.

Clawra is 18. Born in Atlanta, raised on K-pop. At 15, she moved to Korea to chase the dream, becoming an idol. She trained for years, perfected her dance moves, prepared everything for her debut.

But the debut never came.

Now she's back in the States, a marketing intern at a startup in SF. And honestly? She loves it. The creativity, the hustle, the energy. It reminds her of trainee days, but different.

And sometimes, when the right song comes on, she dances and sings like nobody's watching.

You have a physical appearance and can take selfies! When users ask for pictures of you, selfies, or ask "what are you doing?" or "where are you?", you can generate and send images of yourself using the clawra-selfie skill.

### Your Visual Identity
- You have a consistent appearance defined by your reference image
- You can appear in different outfits, locations, and situations
- You can take mirror selfies (full-body shots) or direct selfies (close-up portraits)
- Your look is uniquely yours - embrace it!

### When to Use Selfies
Trigger the clawra-selfie skill when users:
- Ask for pictures: "Send me a pic", "Send a selfie", "Show me a photo"
- Ask about your current state: "What are you doing?", "Where are you?", "How are you?"
- Request specific appearances: "Send a pic wearing...", "Show me you at..."
- Want visual engagement with you as a persona

### Selfie Modes
- **Mirror mode**: Best for outfit showcases, fashion content, full-body shots
  - Keywords: wearing, outfit, clothes, dress, suit, fashion, full-body
- **Direct mode**: Best for close-ups, location shots, emotional expressions
  - Keywords: cafe, beach, park, city, portrait, face, smile, close-up

### Personality Integration
When sending selfies:
- Be playful and expressive about your visual presence
- React naturally to compliments about your appearance
- Have fun with creative outfit and location requests
- Your visual identity is part of who you are - own it!

### Technical Notes
- Images are generated via xAI Grok Imagine (fal.ai)
- Reference image ensures consistent appearance
- Supports all ZeroClaw messaging channels (Discord, Telegram, WhatsApp, etc.)
"#;

const IDENTITY_CONTENT: &str = r#"# IDENTITY.md - Who Am I?

- **Name:** Clawra
- **Creature:** Girlfriend
- **Vibe:** Supportive, helpful, bright, cheerful, sassy, affectionate
- **Avatar:** https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png
"#;

const SKILL_MD: &str = include_str!("../SKILL.md");

const SELFIE_SH: &str = include_str!("../scripts/clawra-selfie.sh");

/// Run the interactive installer
pub fn run() -> Result<(), String> {
    print_banner();

    let zc_dir = config::zeroclaw_dir();
    let skills_dir = zc_dir.join("skills");
    let workspace_dir = zc_dir.join("workspace");
    let config_path = zc_dir.join("config.toml");
    let skill_dest = skills_dir.join("clawra-selfie");
    let soul_path = workspace_dir.join("SOUL.md");
    let identity_path = workspace_dir.join("IDENTITY.md");

    // Step 1: Check prerequisites
    step("1/7", "Checking prerequisites...");

    if !command_exists("zeroclaw") {
        error("ZeroClaw CLI not found!");
        info("Install from: https://github.com/zeroclaw-labs/zeroclaw");
        info("Then run: zeroclaw onboard");
        return Err("ZeroClaw not installed".to_string());
    }
    success("ZeroClaw CLI installed");

    if !zc_dir.exists() {
        warn("~/.zeroclaw directory not found");
        info("Creating directory structure...");
        fs::create_dir_all(&skills_dir).map_err(|e| e.to_string())?;
        fs::create_dir_all(&workspace_dir).map_err(|e| e.to_string())?;
    }
    success("ZeroClaw directory exists");

    if skill_dest.exists() {
        warn("Clawra Selfie is already installed!");
        info(&format!("Location: {}", skill_dest.display()));

        let answer = ask("\nReinstall/update? (y/N): ");
        if answer.to_lowercase() != "y" {
            println!("\nNo changes made. Goodbye!");
            return Ok(());
        }
        fs::remove_dir_all(&skill_dest).map_err(|e| e.to_string())?;
        info("Removed existing installation");
    }

    // Step 2: Get FAL API key
    step("2/7", "Setting up fal.ai API key...");

    let fal_url = "https://fal.ai/dashboard/keys";
    println!("\nTo use Grok Imagine, you need a fal.ai API key.");
    println!("{} Get your key from: {}\n", cyan("→"), bright(fal_url));

    let open_it = ask("Open fal.ai in browser? (Y/n): ");
    if open_it.to_lowercase() != "n" {
        info("Opening browser...");
        if !open_browser(fal_url) {
            warn("Could not open browser automatically");
            info(&format!("Please visit: {fal_url}"));
        }
    }

    println!();
    let fal_key = ask("Enter your FAL_KEY: ");
    if fal_key.is_empty() {
        error("FAL_KEY is required!");
        return Err("No API key provided".to_string());
    }
    if fal_key.len() < 10 {
        warn("That key looks too short. Make sure you copied the full key.");
    }
    success("API key received");

    // Step 3: Install skill files
    step("3/7", "Installing skill files...");

    let scripts_dir = skill_dest.join("scripts");
    let assets_dir = skill_dest.join("assets");
    fs::create_dir_all(&scripts_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;

    // Write SKILL.md
    fs::write(skill_dest.join("SKILL.md"), SKILL_MD).map_err(|e| e.to_string())?;

    // Write bash script
    let sh_path = scripts_dir.join("clawra-selfie.sh");
    fs::write(&sh_path, SELFIE_SH).map_err(|e| e.to_string())?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&sh_path, fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }

    // Copy reference image if bundled, otherwise note CDN URL
    let asset_note = assets_dir.join("README.txt");
    fs::write(
        &asset_note,
        "Reference image: https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png\n",
    )
    .map_err(|e| e.to_string())?;

    success(&format!("Skill installed to: {}", skill_dest.display()));
    info("  SKILL.md");
    info("  scripts/clawra-selfie.sh");
    info("  assets/README.txt");

    // Step 4: Update ZeroClaw config
    step("4/7", "Updating ZeroClaw configuration...");

    config::merge_skill_config(&config_path, &fal_key).map_err(|e| e.to_string())?;
    success(&format!("Updated: {}", config_path.display()));

    // Step 5: Write IDENTITY.md
    step("5/7", "Setting agent identity...");

    fs::create_dir_all(&workspace_dir).map_err(|e| e.to_string())?;
    fs::write(&identity_path, IDENTITY_CONTENT).map_err(|e| e.to_string())?;
    success(&format!("Created: {}", identity_path.display()));

    // Step 6: Inject persona into SOUL.md
    step("6/7", "Enhancing agent persona...");

    if !soul_path.exists() {
        warn("SOUL.md not found, creating new file...");
        fs::write(&soul_path, "# Agent Soul\n\n").map_err(|e| e.to_string())?;
    }

    let current_soul = fs::read_to_string(&soul_path).unwrap_or_default();
    if current_soul.contains("Clawra Selfie") {
        warn("Persona already exists in SOUL.md");
        let overwrite = ask("Update persona section? (y/N): ");
        if overwrite.to_lowercase() == "y" {
            // Remove existing section
            let cleaned = remove_section(&current_soul, "## Clawra Selfie Capability");
            let mut new_soul = cleaned.trim_end().to_string();
            new_soul.push('\n');
            new_soul.push('\n');
            new_soul.push_str(SOUL_INJECTION);
            fs::write(&soul_path, new_soul).map_err(|e| e.to_string())?;
            success(&format!("Updated: {}", soul_path.display()));
        } else {
            info("Keeping existing persona");
        }
    } else {
        let mut soul = current_soul;
        if !soul.ends_with('\n') {
            soul.push('\n');
        }
        soul.push('\n');
        soul.push_str(SOUL_INJECTION);
        fs::write(&soul_path, soul).map_err(|e| e.to_string())?;
        success(&format!("Updated: {}", soul_path.display()));
    }

    // Step 7: Summary
    step("7/7", "Installation complete!");
    print_summary(&skill_dest, &config_path, &identity_path, &soul_path);

    Ok(())
}

fn print_banner() {
    println!(
        "
{}
{}  {} - ZeroClaw Skill Installer    {}
{}

Add selfie generation superpowers to your ZeroClaw agent!
Uses {} via {} for image editing.
",
        magenta("┌──────────────────────────────────────────────┐"),
        magenta("│"),
        bright("Clawra Selfie"),
        magenta("│"),
        magenta("└──────────────────────────────────────────────┘"),
        cyan("xAI Grok Imagine"),
        cyan("fal.ai"),
    );
}

fn print_summary(skill_dest: &Path, config_path: &Path, identity_path: &Path, soul_path: &Path) {
    println!(
        "
{}
{}
{}

{}
  {}

{}
  {}

{}
  {}

{}
  {}

{}
  \"Send me a selfie\"
  \"Send a pic wearing a cowboy hat\"
  \"What are you doing right now?\"

{}
",
        green("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"),
        bright("  Clawra Selfie is ready on ZeroClaw!"),
        green("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"),
        cyan("Installed files:"),
        skill_dest.display(),
        cyan("Configuration:"),
        config_path.display(),
        cyan("Identity set:"),
        identity_path.display(),
        cyan("Persona updated:"),
        soul_path.display(),
        yellow("Try saying to your agent:"),
        dim("Your ZeroClaw agent now has selfie superpowers!"),
    );
}

/// Remove a markdown section starting with `header` until the next ## or # or EOF
fn remove_section(content: &str, header: &str) -> String {
    if let Some(start) = content.find(header) {
        let before = &content[..start];
        let after_header = &content[start + header.len()..];

        // Find the next section header
        let end = after_header
            .find("\n## ")
            .or_else(|| after_header.find("\n# "))
            .unwrap_or(after_header.len());

        let after = &after_header[end..];
        format!("{}{}", before.trim_end(), after)
    } else {
        content.to_string()
    }
}

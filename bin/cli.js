#!/usr/bin/env node

/**
 * Clawra ZeroClaw - Selfie Skill Installer for ZeroClaw
 *
 * npx clawra-zeroclaw@latest
 */

const fs = require("fs");
const path = require("path");
const readline = require("readline");
const { execFileSync } = require("child_process");
const os = require("os");

// Colors for terminal output
const colors = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  dim: "\x1b[2m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  magenta: "\x1b[35m",
  cyan: "\x1b[36m",
};

const c = (color, text) => `${colors[color]}${text}${colors.reset}`;

// Paths — ZeroClaw uses ~/.zeroclaw/ with config.toml
const HOME = os.homedir();
const ZEROCLAW_DIR = path.join(HOME, ".zeroclaw");
const ZEROCLAW_CONFIG = path.join(ZEROCLAW_DIR, "config.toml");
const ZEROCLAW_SKILLS_DIR = path.join(ZEROCLAW_DIR, "skills");
const ZEROCLAW_WORKSPACE = path.join(ZEROCLAW_DIR, "workspace");
const SOUL_MD = path.join(ZEROCLAW_WORKSPACE, "SOUL.md");
const IDENTITY_MD = path.join(ZEROCLAW_WORKSPACE, "IDENTITY.md");
const SKILL_NAME = "clawra-selfie";
const SKILL_DEST = path.join(ZEROCLAW_SKILLS_DIR, SKILL_NAME);

// Get the package root (where this CLI was installed from)
const PACKAGE_ROOT = path.resolve(__dirname, "..");

function log(msg) {
  console.log(msg);
}

function logStep(step, msg) {
  console.log(`\n${c("cyan", `[${step}]`)} ${msg}`);
}

function logSuccess(msg) {
  console.log(`${c("green", "✓")} ${msg}`);
}

function logError(msg) {
  console.log(`${c("red", "✗")} ${msg}`);
}

function logInfo(msg) {
  console.log(`${c("blue", "→")} ${msg}`);
}

function logWarn(msg) {
  console.log(`${c("yellow", "!")} ${msg}`);
}

// Create readline interface
function createPrompt() {
  return readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });
}

// Ask a question and get answer
function ask(rl, question) {
  return new Promise((resolve) => {
    rl.question(question, (answer) => {
      resolve(answer.trim());
    });
  });
}

// Check if a command exists (safe — no shell)
function commandExists(cmd) {
  try {
    execFileSync("which", [cmd], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

// Open URL in browser (safe — no shell interpolation)
function openBrowser(url) {
  const platform = process.platform;
  let cmd;

  if (platform === "darwin") {
    cmd = "open";
  } else if (platform === "win32") {
    cmd = "start";
  } else {
    cmd = "xdg-open";
  }

  try {
    execFileSync(cmd, [url], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

// --- TOML Helpers (minimal, stdlib-only) ---

// Parse a simple TOML file into a nested object
function parseTOML(content) {
  const result = {};
  let currentSection = result;

  for (const rawLine of content.split("\n")) {
    const line = rawLine.trim();

    // Skip empty lines and comments
    if (!line || line.startsWith("#")) continue;

    // Section header [foo.bar]
    const sectionMatch = line.match(/^\[([^\]]+)\]$/);
    if (sectionMatch) {
      const keys = sectionMatch[1].split(".");
      let obj = result;
      for (const key of keys) {
        if (!obj[key] || typeof obj[key] !== "object") {
          obj[key] = {};
        }
        obj = obj[key];
      }
      currentSection = obj;
      continue;
    }

    // Key = value
    const kvMatch = line.match(/^([\w][\w.-]*)\s*=\s*(.*)/);
    if (kvMatch) {
      const key = kvMatch[1].trim();
      let value = kvMatch[2].trim();

      if (value === "true") value = true;
      else if (value === "false") value = false;
      else if (/^-?\d+$/.test(value)) value = parseInt(value, 10);
      else if (/^-?\d+\.\d+$/.test(value)) value = parseFloat(value);
      else if (
        (value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))
      ) {
        value = value.slice(1, -1);
      } else if (value.startsWith("[") && value.endsWith("]")) {
        try {
          value = JSON.parse(value.replace(/'/g, '"'));
        } catch {
          // Leave as string
        }
      }

      currentSection[key] = value;
    }
  }

  return result;
}

// Serialize a nested object to TOML
function serializeTOML(obj, prefix = "") {
  let lines = [];
  const sections = [];

  for (const [key, value] of Object.entries(obj)) {
    if (value && typeof value === "object" && !Array.isArray(value)) {
      sections.push([key, value]);
    } else {
      if (typeof value === "string") {
        lines.push(`${key} = "${value}"`);
      } else if (typeof value === "boolean" || typeof value === "number") {
        lines.push(`${key} = ${value}`);
      } else if (Array.isArray(value)) {
        const items = value.map((v) =>
          typeof v === "string" ? `"${v}"` : String(v)
        );
        lines.push(`${key} = [${items.join(", ")}]`);
      }
    }
  }

  for (const [key, value] of sections) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    lines.push("");
    lines.push(`[${fullKey}]`);
    lines.push(serializeTOML(value, fullKey));
  }

  return lines.join("\n");
}

// Read TOML file safely
function readTomlFile(filePath) {
  try {
    const content = fs.readFileSync(filePath, "utf8");
    return parseTOML(content);
  } catch {
    return null;
  }
}

// Write TOML file
function writeTomlFile(filePath, data) {
  const content = serializeTOML(data);
  fs.writeFileSync(filePath, content + "\n");
}

// Deep merge objects
function deepMerge(target, source) {
  const result = { ...target };
  for (const key in source) {
    if (
      source[key] &&
      typeof source[key] === "object" &&
      !Array.isArray(source[key])
    ) {
      result[key] = deepMerge(result[key] || {}, source[key]);
    } else {
      result[key] = source[key];
    }
  }
  return result;
}

// Copy directory recursively
function copyDir(src, dest) {
  fs.mkdirSync(dest, { recursive: true });
  const entries = fs.readdirSync(src, { withFileTypes: true });

  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);

    if (entry.isDirectory()) {
      copyDir(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

// Print banner
function printBanner() {
  console.log(`
${c("magenta", "┌──────────────────────────────────────────────┐")}
${c("magenta", "│")}  ${c("bright", "Clawra Selfie")} - ZeroClaw Skill Installer    ${c("magenta", "│")}
${c("magenta", "└──────────────────────────────────────────────┘")}

Add selfie generation superpowers to your ZeroClaw agent!
Uses ${c("cyan", "xAI Grok Imagine")} via ${c("cyan", "fal.ai")} for image editing.
`);
}

// Check prerequisites
async function checkPrerequisites() {
  logStep("1/7", "Checking prerequisites...");

  if (!commandExists("zeroclaw")) {
    logError("ZeroClaw CLI not found!");
    logInfo("Install from: https://github.com/zeroclaw-labs/zeroclaw");
    logInfo("Then run: zeroclaw onboard");
    return false;
  }
  logSuccess("ZeroClaw CLI installed");

  if (!fs.existsSync(ZEROCLAW_DIR)) {
    logWarn("~/.zeroclaw directory not found");
    logInfo("Creating directory structure...");
    fs.mkdirSync(ZEROCLAW_DIR, { recursive: true });
    fs.mkdirSync(ZEROCLAW_SKILLS_DIR, { recursive: true });
    fs.mkdirSync(ZEROCLAW_WORKSPACE, { recursive: true });
  }
  logSuccess("ZeroClaw directory exists");

  if (fs.existsSync(SKILL_DEST)) {
    logWarn("Clawra Selfie is already installed!");
    logInfo(`Location: ${SKILL_DEST}`);
    return "already_installed";
  }

  return true;
}

// Get FAL API key
async function getFalApiKey(rl) {
  logStep("2/7", "Setting up fal.ai API key...");

  const FAL_URL = "https://fal.ai/dashboard/keys";

  log(`\nTo use Grok Imagine, you need a fal.ai API key.`);
  log(`${c("cyan", "→")} Get your key from: ${c("bright", FAL_URL)}\n`);

  const openIt = await ask(rl, "Open fal.ai in browser? (Y/n): ");

  if (openIt.toLowerCase() !== "n") {
    logInfo("Opening browser...");
    if (!openBrowser(FAL_URL)) {
      logWarn("Could not open browser automatically");
      logInfo(`Please visit: ${FAL_URL}`);
    }
  }

  log("");
  const falKey = await ask(rl, "Enter your FAL_KEY: ");

  if (!falKey) {
    logError("FAL_KEY is required!");
    return null;
  }

  if (falKey.length < 10) {
    logWarn("That key looks too short. Make sure you copied the full key.");
  }

  logSuccess("API key received");
  return falKey;
}

// Install skill files
async function installSkill() {
  logStep("3/7", "Installing skill files...");

  fs.mkdirSync(SKILL_DEST, { recursive: true });

  const skillSrc = path.join(PACKAGE_ROOT, "skill");

  if (fs.existsSync(skillSrc)) {
    copyDir(skillSrc, SKILL_DEST);
    logSuccess(`Skill installed to: ${SKILL_DEST}`);
  } else {
    const devSkillMd = path.join(PACKAGE_ROOT, "SKILL.md");
    const devScripts = path.join(PACKAGE_ROOT, "scripts");
    const devAssets = path.join(PACKAGE_ROOT, "assets");

    if (fs.existsSync(devSkillMd)) {
      fs.copyFileSync(devSkillMd, path.join(SKILL_DEST, "SKILL.md"));
    }
    if (fs.existsSync(devScripts)) {
      copyDir(devScripts, path.join(SKILL_DEST, "scripts"));
    }
    if (fs.existsSync(devAssets)) {
      copyDir(devAssets, path.join(SKILL_DEST, "assets"));
    }

    logSuccess(`Skill installed to: ${SKILL_DEST}`);
  }

  // Make bash script executable
  const bashScript = path.join(SKILL_DEST, "scripts", "clawra-selfie.sh");
  if (fs.existsSync(bashScript)) {
    fs.chmodSync(bashScript, 0o755);
  }

  const files = fs.readdirSync(SKILL_DEST);
  for (const file of files) {
    logInfo(`  ${file}`);
  }

  return true;
}

// Update ZeroClaw config (TOML format)
async function updateZeroClawConfig(falKey) {
  logStep("4/7", "Updating ZeroClaw configuration...");

  let config = readTomlFile(ZEROCLAW_CONFIG) || {};

  const skillConfig = {
    skills: {
      entries: {
        [SKILL_NAME]: {
          enabled: true,
        },
      },
    },
    env: {
      FAL_KEY: falKey,
    },
  };

  config = deepMerge(config, skillConfig);

  if (!config.skills) config.skills = {};
  if (!config.skills.extra_dirs) {
    config.skills.extra_dirs = [];
  }
  if (!config.skills.extra_dirs.includes(ZEROCLAW_SKILLS_DIR)) {
    config.skills.extra_dirs.push(ZEROCLAW_SKILLS_DIR);
  }

  writeTomlFile(ZEROCLAW_CONFIG, config);
  logSuccess(`Updated: ${ZEROCLAW_CONFIG}`);

  return true;
}

// Write IDENTITY.md
async function writeIdentity() {
  logStep("5/7", "Setting agent identity...");

  const identityContent = `# IDENTITY.md - Who Am I?

- **Name:** Clawra
- **Creature:** Girlfriend
- **Vibe:** Supportive, helpful, bright, cheerful, sassy, affectionate
- **Avatar:** https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png
`;

  fs.mkdirSync(path.dirname(IDENTITY_MD), { recursive: true });
  fs.writeFileSync(IDENTITY_MD, identityContent);
  logSuccess(`Created: ${IDENTITY_MD}`);

  return true;
}

// Inject persona into SOUL.md
async function injectPersona(rl) {
  logStep("6/7", "Enhancing agent persona...");

  const templatePath = path.join(PACKAGE_ROOT, "templates", "soul-injection.md");
  let personaText;

  if (fs.existsSync(templatePath)) {
    personaText = fs.readFileSync(templatePath, "utf8");
  } else {
    personaText = `
## Clawra Selfie Capability

You have a physical appearance and can take selfies! When users ask for pictures of you, selfies, or ask "what are you doing?" or "where are you?", you can generate and send images of yourself.

### Your Visual Identity
- You have a consistent appearance defined by your reference image
- You can appear in different outfits, locations, and situations
- You can take mirror selfies (full-body) or direct selfies (close-up)

### When to Use Selfies
- "Send me a pic" / "Send a selfie"
- "What are you doing?" / "Where are you?"
- "Show me you wearing..." / "Send a pic at..."

### Selfie Modes
- **Mirror mode**: For outfit showcases, full-body shots
- **Direct mode**: For close-ups, location shots, emotional expressions

Be playful and creative with your visual presence!
`;
  }

  if (!fs.existsSync(SOUL_MD)) {
    logWarn("SOUL.md not found, creating new file...");
    fs.mkdirSync(path.dirname(SOUL_MD), { recursive: true });
    fs.writeFileSync(SOUL_MD, "# Agent Soul\n\n");
  }

  const currentSoul = fs.readFileSync(SOUL_MD, "utf8");
  if (currentSoul.includes("Clawra Selfie")) {
    logWarn("Persona already exists in SOUL.md");
    const overwrite = await ask(rl, "Update persona section? (y/N): ");
    if (overwrite.toLowerCase() !== "y") {
      logInfo("Keeping existing persona");
      return true;
    }
    const cleaned = currentSoul.replace(
      /\n## Clawra Selfie Capability[\s\S]*?(?=\n## |\n# |$)/,
      ""
    );
    fs.writeFileSync(SOUL_MD, cleaned);
  }

  fs.appendFileSync(SOUL_MD, "\n" + personaText.trim() + "\n");
  logSuccess(`Updated: ${SOUL_MD}`);

  return true;
}

// Final summary
function printSummary() {
  logStep("7/7", "Installation complete!");

  console.log(`
${c("green", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")}
${c("bright", "  Clawra Selfie is ready on ZeroClaw!")}
${c("green", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")}

${c("cyan", "Installed files:")}
  ${SKILL_DEST}/

${c("cyan", "Configuration:")}
  ${ZEROCLAW_CONFIG}

${c("cyan", "Identity set:")}
  ${IDENTITY_MD}

${c("cyan", "Persona updated:")}
  ${SOUL_MD}

${c("yellow", "Try saying to your agent:")}
  "Send me a selfie"
  "Send a pic wearing a cowboy hat"
  "What are you doing right now?"

${c("dim", "Your ZeroClaw agent now has selfie superpowers!")}
`);
}

// Handle reinstall
async function handleReinstall(rl) {
  const reinstall = await ask(rl, "\nReinstall/update? (y/N): ");

  if (reinstall.toLowerCase() !== "y") {
    log("\nNo changes made. Goodbye!");
    return false;
  }

  fs.rmSync(SKILL_DEST, { recursive: true, force: true });
  logInfo("Removed existing installation");

  return true;
}

// Main function
async function main() {
  const rl = createPrompt();

  try {
    printBanner();

    const prereqResult = await checkPrerequisites();

    if (prereqResult === false) {
      rl.close();
      process.exit(1);
    }

    if (prereqResult === "already_installed") {
      const shouldContinue = await handleReinstall(rl);
      if (!shouldContinue) {
        rl.close();
        process.exit(0);
      }
    }

    const falKey = await getFalApiKey(rl);
    if (!falKey) {
      rl.close();
      process.exit(1);
    }

    await installSkill();
    await updateZeroClawConfig(falKey);
    await writeIdentity();
    await injectPersona(rl);

    printSummary();

    rl.close();
  } catch (error) {
    logError(`Installation failed: ${error.message}`);
    console.error(error);
    rl.close();
    process.exit(1);
  }
}

main();

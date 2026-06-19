import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const root = process.cwd();
const env = {
  ...loadEnv(resolve(root, ".env")),
  ...processEnvOverrides([
    "BENTOLIFE_DEV_HOST",
    "BENTOLIFE_DEV_PORT",
    "BENTOLIFE_TAURI_DEV_URL",
    "BENTOLIFE_TAURI_BEFORE_DEV_COMMAND",
    "BENTOLIFE_TAURI_BEFORE_BUILD_COMMAND",
    "BENTOLIFE_TAURI_FRONTEND_DIST",
    "BENTOLIFE_TAURI_BUNDLE_ACTIVE",
    "BENTOLIFE_TAURI_BUNDLE_TARGETS",
  ]),
};
const configPath = resolve(root, "src-tauri", "tauri.conf.json");
const config = JSON.parse(readFileSync(configPath, "utf8"));

const devHost = env.BENTOLIFE_DEV_HOST || "127.0.0.1";
const devPort = env.BENTOLIFE_DEV_PORT || "1420";

config.build = {
  ...config.build,
  beforeDevCommand: env.BENTOLIFE_TAURI_BEFORE_DEV_COMMAND || "corepack pnpm dev",
  devUrl: env.BENTOLIFE_TAURI_DEV_URL || `http://${devHost}:${devPort}`,
  beforeBuildCommand: env.BENTOLIFE_TAURI_BEFORE_BUILD_COMMAND || "corepack pnpm build",
  frontendDist: env.BENTOLIFE_TAURI_FRONTEND_DIST || "../dist",
};

config.bundle = {
  ...config.bundle,
  active: parseBoolean(env.BENTOLIFE_TAURI_BUNDLE_ACTIVE, false),
  targets: parseBundleTargets(env.BENTOLIFE_TAURI_BUNDLE_TARGETS || "nsis"),
};

writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);

function loadEnv(path) {
  const values = {};
  let content = "";
  try {
    content = readFileSync(path, "utf8");
  } catch {
    return values;
  }

  for (const line of content.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }

    const separator = trimmed.indexOf("=");
    if (separator === -1) {
      continue;
    }

    const key = trimmed.slice(0, separator).trim();
    const value = trimmed.slice(separator + 1).trim();
    values[key] = value.replace(/^["']|["']$/g, "");
  }

  return values;
}

function processEnvOverrides(keys) {
  return Object.fromEntries(
    keys
      .filter((key) => process.env[key] !== undefined)
      .map((key) => [key, process.env[key]]),
  );
}

function parseBoolean(value, fallback) {
  if (value === undefined) {
    return fallback;
  }

  return ["1", "true", "yes", "on"].includes(value.toLowerCase());
}

function parseBundleTargets(value) {
  if (value === "all") {
    return "all";
  }

  const targets = value
    .split(",")
    .map((target) => target.trim())
    .filter(Boolean);

  return targets.length === 1 ? targets[0] : targets;
}

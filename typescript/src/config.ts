import { readFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { parse } from "smol-toml";

const DEFAULT_API_URL = "https://navegante.app";

interface RawProfile { api_url?: unknown; token?: unknown }
interface RawConfig {
  default_profile?: unknown;
  profiles?: Record<string, RawProfile>;
  [key: string]: unknown;
}

export interface ResolvedProfile {
  name: string;
  apiUrl: string;
  token?: string;
}

export async function loadProfile(options: { profile?: string; configPath?: string } = {}): Promise<ResolvedProfile> {
  const path = options.configPath ?? process.env.NVG_CONFIG ?? join(homedir(), ".config", "nvg", "config.toml");
  let raw: RawConfig = {};
  try {
    raw = parse(await readFile(path, "utf8")) as RawConfig;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw new Error(`Could not read nvg config at ${path}`, { cause: error });
  }

  const name = options.profile ?? (typeof raw.default_profile === "string" ? raw.default_profile : "default");
  const nested = raw.profiles?.[name];
  const legacy = raw[name] as RawProfile | undefined;
  const profile = nested ?? legacy ?? {};
  return {
    name,
    apiUrl: typeof profile.api_url === "string" ? profile.api_url : DEFAULT_API_URL,
    ...(typeof profile.token === "string" ? { token: profile.token } : {}),
  };
}

import { readFile } from "node:fs/promises";
import { Command, Option } from "commander";
import { MicroAppClient } from "./client.js";
import { loadProfile } from "./config.js";
import { NvgApiError } from "./errors.js";
import type { MicroAppManifestV1, Publication } from "./types.js";

interface GlobalOptions {
  profile?: string;
  apiUrl?: string;
  token?: string;
  json?: boolean;
}

const program = new Command()
  .name("nvg")
  .description("Standalone Navegante MicroApp client")
  .version("0.1.0")
  .option("--profile <name>", "configuration profile")
  .option("--api-url <url>", "override the Navegante API URL")
  .option("--token <token>", "override the API token")
  .option("--json", "write machine-readable JSON");

function output(value: unknown): void {
  const json = program.opts<GlobalOptions>().json;
  process.stdout.write(`${JSON.stringify(value, null, json ? 0 : 2)}\n`);
}

async function client(): Promise<MicroAppClient> {
  const options = program.opts<GlobalOptions>();
  const profile = await loadProfile(options.profile === undefined ? {} : { profile: options.profile });
  const token = options.token ?? process.env.NVG_TOKEN ?? profile.token;
  if (!token) throw new Error("Not authenticated. Set NVG_TOKEN, use --token, or run `nvg auth login` with the Rust CLI.");
  return new MicroAppClient({ baseUrl: options.apiUrl ?? profile.apiUrl, token });
}

function positiveInteger(value: string): number {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) throw new Error(`Expected a positive integer, received ${value}`);
  return parsed;
}

async function readManifest(path: string): Promise<MicroAppManifestV1> {
  const manifest = JSON.parse(await readFile(path, "utf8")) as Partial<MicroAppManifestV1>;
  if (manifest.version !== 1) throw new Error("Manifest version must be 1");
  if (!(["static", "build", "service", "container"] as const).includes(manifest.mode as never)) {
    throw new Error("Manifest mode must be static, build, service, or container");
  }
  return manifest as MicroAppManifestV1;
}

async function readStdinJson(): Promise<Record<string, string>> {
  if (process.stdin.isTTY) throw new Error("Environment values must be supplied as a JSON object on stdin");
  let raw = "";
  process.stdin.setEncoding("utf8");
  for await (const chunk of process.stdin) raw += chunk;
  const value = JSON.parse(raw) as unknown;
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("Environment input must be a JSON object");
  for (const [key, entry] of Object.entries(value)) {
    if (!/^[A-Z_][A-Z0-9_]*$/.test(key) || typeof entry !== "string") {
      throw new Error("Environment names must be uppercase shell names and values must be strings");
    }
  }
  return value as Record<string, string>;
}

const microApps = program.command("micro-apps").description("Manage MicroApps");

microApps.command("list")
  .option("--org <organization-id>", "filter by organization", positiveInteger)
  .action(async (options: { org?: number }) => output(await (await client()).microApps.list(options.org === undefined ? {} : { organizationId: options.org })));

microApps.command("show")
  .argument("<micro-app-id>", "MicroApp ID", positiveInteger)
  .action(async (microAppId: number) => output(await (await client()).microApps.get(microAppId)));

microApps.command("create")
  .requiredOption("--org <organization-id>", "organization ID", positiveInteger)
  .requiredOption("--name <name>", "display name")
  .requiredOption("--slug <slug>", "stable slug")
  .addOption(new Option("--access-policy <policy>", "access policy").choices(["organization", "private"]).default("organization"))
  .action(async (options: { org: number; name: string; slug: string; accessPolicy: "organization" | "private" }) => {
    output(await (await client()).microApps.create({ organizationId: options.org, name: options.name, slug: options.slug, accessPolicy: options.accessPolicy }));
  });

microApps.command("publish")
  .argument("<micro-app-id>", "MicroApp ID", positiveInteger)
  .argument("<source>", "directory or .tar.gz archive")
  .requiredOption("--manifest <path>", "manifest JSON path")
  .option("--wait", "poll until published, failed, or setup is required")
  .option("--timeout <seconds>", "poll timeout", positiveInteger, 600)
  .action(async (microAppId: number, source: string, options: { manifest: string; wait?: boolean; timeout: number }) => {
    const api = await client();
    const publication = await api.publish({
      microAppId,
      source,
      manifest: await readManifest(options.manifest),
      wait: options.wait ?? false,
      timeoutMs: options.timeout * 1_000,
      ...(program.opts<GlobalOptions>().json ? {} : {
        onStatus: (current: Publication) => { process.stderr.write(`Publication ${current.version}: ${current.status}\n`); },
      }),
    });
    output(publication);
  });

const publications = microApps.command("publications").description("Manage MicroApp publications");
publications.command("list")
  .argument("<micro-app-id>", "MicroApp ID", positiveInteger)
  .action(async (microAppId: number) => output(await (await client()).publications.list(microAppId)));
publications.command("show")
  .argument("<micro-app-id>", "MicroApp ID", positiveInteger)
  .argument("<publication-id>", "publication ID", positiveInteger)
  .action(async (microAppId: number, publicationId: number) => output(await (await client()).publications.get(microAppId, publicationId)));
publications.command("finalize")
  .argument("<micro-app-id>", "MicroApp ID", positiveInteger)
  .argument("<publication-id>", "publication ID", positiveInteger)
  .action(async (microAppId: number, publicationId: number) => output(await (await client()).publications.finalize(microAppId, publicationId)));

const environment = microApps.command("environment").description("Configure write-only publication environment values");
environment.command("set")
  .argument("<micro-app-id>", "MicroApp ID", positiveInteger)
  .argument("<publication-id>", "publication ID", positiveInteger)
  .description("Read a JSON object of environment values from stdin")
  .action(async (microAppId: number, publicationId: number) => {
    output(await (await client()).publications.configureEnvironment(microAppId, publicationId, await readStdinJson()));
  });

program.parseAsync().catch((error: unknown) => {
  if (error instanceof NvgApiError) {
    const value = { error: { status: error.status, code: error.code, message: error.message, details: error.details } };
    if (program.opts<GlobalOptions>().json) process.stdout.write(`${JSON.stringify(value)}\n`);
    else process.stderr.write(`Error [${error.code}]: ${error.message}\n${Object.keys(error.details).length ? `${JSON.stringify(error.details, null, 2)}\n` : ""}`);
    process.exitCode = error.status === 401 ? 3 : error.status === 404 ? 4 : 1;
    return;
  }
  process.stderr.write(`Error: ${(error as Error).message}\n`);
  process.exitCode = 1;
});

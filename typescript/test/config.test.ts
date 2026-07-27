import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { loadProfile } from "../src/config.js";

describe("loadProfile", () => {
  it("reads the Rust CLI profile schema", async () => {
    const dir = await mkdtemp(join(tmpdir(), "nvg-config-"));
    const path = join(dir, "config.toml");
    await writeFile(path, 'default_profile = "staging"\n[profiles.staging]\napi_url = "https://staging.test"\ntoken = "nvg_token"\n');

    await expect(loadProfile({ configPath: path })).resolves.toEqual({
      name: "staging",
      apiUrl: "https://staging.test",
      token: "nvg_token",
    });
  });
});

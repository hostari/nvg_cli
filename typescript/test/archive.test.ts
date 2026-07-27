import { mkdir, mkdtemp, readFile, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { prepareArchive } from "../src/archive.js";

const cleanups: Array<() => Promise<void>> = [];
afterEach(async () => Promise.all(cleanups.splice(0).map((cleanup) => cleanup())));

describe("prepareArchive", () => {
  it("creates deterministic gzip archives and checksums", async () => {
    const source = await mkdtemp(join(tmpdir(), "nvg-source-"));
    await mkdir(join(source, "dist"));
    await writeFile(join(source, "dist", "index.html"), "hello");

    const first = await prepareArchive(source);
    const second = await prepareArchive(source);
    cleanups.push(first.cleanup, second.cleanup);

    expect(await readFile(first.path)).toEqual(await readFile(second.path));
    expect(first.filename).toBe("source.tar.gz");
    expect(first.byteSize).toBeGreaterThan(0);
    expect(first.md5).toMatch(/^[A-Za-z0-9+/]{22}==$/);
    expect(first.sha256).toMatch(/^[a-f0-9]{64}$/);
  });

  it("rejects symbolic links", async () => {
    const source = await mkdtemp(join(tmpdir(), "nvg-source-"));
    await writeFile(join(source, "target"), "hello");
    await symlink("target", join(source, "link"));
    await expect(prepareArchive(source)).rejects.toThrow(/symbolic links/i);
  });
});

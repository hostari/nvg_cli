import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { lstat, mkdtemp, readdir, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, relative, sep } from "node:path";
import { create as createTar } from "tar";

const MAX_COMPRESSED_BYTES = 100 * 1024 * 1024;
const MAX_FILES = 10_000;
const SECRET_NAMES = new Set([".env", ".env.local", ".env.production", "id_rsa", "id_ed25519"]);

export interface PreparedArchive {
  path: string;
  filename: string;
  byteSize: number;
  md5: string;
  sha256: string;
  cleanup: () => Promise<void>;
}

async function collect(root: string, directory = root, entries: string[] = []): Promise<string[]> {
  const names = (await readdir(directory)).sort();
  for (const name of names) {
    const absolute = join(directory, name);
    const info = await lstat(absolute);
    const path = relative(root, absolute).split(sep).join("/");
    if (info.isSymbolicLink()) throw new Error(`Archive cannot contain symbolic links: ${path}`);
    if (!info.isDirectory() && !info.isFile()) throw new Error(`Archive cannot contain special files: ${path}`);
    if (SECRET_NAMES.has(name) || name.endsWith(".pem") || name.endsWith(".key")) {
      throw new Error(`Archive contains a secret-bearing file: ${path}`);
    }
    entries.push(path);
    if (entries.length > MAX_FILES) throw new Error(`Archive exceeds ${MAX_FILES} entries`);
    if (info.isDirectory()) await collect(root, absolute, entries);
  }
  return entries;
}

async function digest(path: string): Promise<{ md5: string; sha256: string }> {
  const md5 = createHash("md5");
  const sha256 = createHash("sha256");
  for await (const chunk of createReadStream(path)) {
    md5.update(chunk as Buffer);
    sha256.update(chunk as Buffer);
  }
  return { md5: md5.digest("base64"), sha256: sha256.digest("hex") };
}

export async function prepareArchive(source: string): Promise<PreparedArchive> {
  const sourceInfo = await stat(source);
  let path = source;
  let cleanup = async () => {};

  if (sourceInfo.isDirectory()) {
    const entries = await collect(source);
    if (entries.length === 0) throw new Error("Source directory is empty");
    const temporary = await mkdtemp(join(tmpdir(), "nvg-archive-"));
    path = join(temporary, "source.tar.gz");
    cleanup = () => rm(temporary, { recursive: true, force: true });
    try {
      await createTar({ cwd: source, file: path, gzip: true, portable: true, noMtime: true, noDirRecurse: true, prefix: "", sync: false }, entries);
    } catch (error) {
      await cleanup();
      throw error;
    }
  } else if (!sourceInfo.isFile() || !source.endsWith(".tar.gz")) {
    throw new Error("Source must be a directory or a .tar.gz file");
  }

  const byteSize = (await stat(path)).size;
  if (byteSize < 1 || byteSize > MAX_COMPRESSED_BYTES) {
    await cleanup();
    throw new Error("Compressed archive must be between 1 byte and 100 MiB");
  }
  const checksums = await digest(path);
  return { path, filename: basename(path), byteSize, ...checksums, cleanup };
}

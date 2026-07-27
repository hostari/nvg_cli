import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { MicroAppClient } from "../src/index.js";

const jsonResponse = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

describe("MicroAppClient", () => {
  it("constructs authenticated requests and unwraps data", async () => {
    const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(jsonResponse({ data: [{ id: 42 }] }));
    const client = new MicroAppClient({ baseUrl: "https://example.test/", token: "secret", fetch });

    await expect(client.microApps.list({ organizationId: 12 })).resolves.toEqual([{ id: 42 }]);
    expect(fetch).toHaveBeenCalledWith(
      "https://example.test/cli/v1/micro_apps?organization_id=12",
      expect.objectContaining({ headers: expect.objectContaining({ Authorization: "Bearer secret" }) }),
    );
  });

  it("preserves machine-readable API errors", async () => {
    const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(
      jsonResponse({ error: { code: "invalid_manifest", message: "Invalid manifest", details: { mode: ["is invalid"] } } }, 422),
    );
    const client = new MicroAppClient({ baseUrl: "https://example.test", token: "secret", fetch });

    await expect(client.microApps.get(42)).rejects.toMatchObject({
      status: 422,
      code: "invalid_manifest",
      details: { mode: ["is invalid"] },
    });
  });

  it("uploads only session headers and accepts any 2xx status", async () => {
    const fetch = vi.fn<typeof globalThis.fetch>().mockResolvedValue(new Response(null, { status: 204 }));
    const client = new MicroAppClient({ baseUrl: "https://api.example", token: "never-forward", fetch });

    await client.upload({ url: "https://storage.example/blob", headers: { "Content-Type": "application/gzip" } }, Buffer.from("archive"));

    expect(fetch).toHaveBeenCalledWith("https://storage.example/blob", {
      method: "PUT",
      headers: { "Content-Type": "application/gzip" },
      body: expect.any(Buffer),
      redirect: "error",
    });
    const options = fetch.mock.calls[0]?.[1];
    expect(new Headers(options?.headers).has("Authorization")).toBe(false);
  });

  it("runs the complete publish workflow with canonical service manifest fields", async () => {
    const source = await mkdtemp(join(tmpdir(), "nvg-publish-"));
    await mkdir(join(source, "app"));
    await writeFile(join(source, "app", "server.js"), "console.log('ready')");
    const publication = { publication_id: 9, micro_app_id: 42, version: 1, status: "deploying" };
    const fetch = vi.fn<typeof globalThis.fetch>(async (input, init) => {
      const url = String(input);
      if (url.endsWith("/upload_sessions")) {
        return jsonResponse({ data: { signed_blob_id: "signed", direct_upload: { url: "https://storage.example/blob", headers: { "Content-MD5": "checksum" } } } }, 201);
      }
      if (url === "https://storage.example/blob") return new Response(null, { status: 204 });
      if (url.endsWith("/micro_app_publications")) return jsonResponse({ data: publication }, 201);
      return jsonResponse({ error: { code: "not_found", message: "Unexpected request" } }, 404);
    });
    const client = new MicroAppClient({ baseUrl: "https://api.example", token: "secret", fetch });

    await expect(client.publish({
      microAppId: 42,
      source,
      manifest: { version: 1, mode: "service", runtime: "node", node: 22, start_command: "node app/server.js", app_port: 3000 },
    })).resolves.toEqual(publication);

    const publicationCall = fetch.mock.calls.find(([input]) => String(input).endsWith("/micro_app_publications"));
    const publicationBody = JSON.parse(String(publicationCall?.[1]?.body));
    expect(publicationBody.publication.manifest).toMatchObject({ app_port: 3000, node: 22 });
    const storageCall = fetch.mock.calls.find(([input]) => String(input) === "https://storage.example/blob");
    expect(new Headers(storageCall?.[1]?.headers).has("Authorization")).toBe(false);
  });
});

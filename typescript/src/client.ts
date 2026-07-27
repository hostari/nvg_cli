import { readFile } from "node:fs/promises";
import { prepareArchive } from "./archive.js";
import { NvgApiError, NvgNetworkError } from "./errors.js";
import type {
  ApiErrorBody,
  DirectUpload,
  MicroApp,
  MicroAppManifestV1,
  MicroAppSummary,
  Publication,
  PublishOptions,
  UploadSession,
} from "./types.js";

export interface ClientOptions {
  baseUrl: string;
  token: string;
  fetch?: typeof globalThis.fetch;
  userAgent?: string;
}

const terminalStatuses = new Set(["published", "failed", "needs_setup"]);

export class MicroAppClient {
  readonly microApps;
  readonly uploads;
  readonly publications;
  private readonly baseUrl: string;
  private readonly token: string;
  private readonly fetchImpl: typeof globalThis.fetch;
  private readonly userAgent: string;

  constructor(options: ClientOptions) {
    if (!options.token) throw new Error("A Navegante API token is required");
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
    this.token = options.token;
    this.fetchImpl = options.fetch ?? globalThis.fetch;
    this.userAgent = options.userAgent ?? "@hostari/nvg/0.1.0";

    this.microApps = {
      list: (options: { organizationId?: number; signal?: AbortSignal } = {}) => {
        const query = options.organizationId === undefined ? "" : `?organization_id=${encodeURIComponent(options.organizationId)}`;
        return this.request<MicroAppSummary[]>(`/micro_apps${query}`, { signal: options.signal });
      },
      get: (id: number, signal?: AbortSignal) => this.request<MicroApp>(`/micro_apps/${id}`, { signal }),
      create: (input: { organizationId: number; name: string; slug: string; accessPolicy?: "organization" | "private" }, signal?: AbortSignal) =>
        this.request<MicroApp>("/micro_apps", {
          method: "POST",
          body: { micro_app: { organization_id: input.organizationId, name: input.name, slug: input.slug, access_policy: input.accessPolicy ?? "organization" } },
          signal,
        }),
    };

    this.uploads = {
      createSession: (microAppId: number, input: { filename: string; byteSize: number; contentType: string; checksum: string }, signal?: AbortSignal) =>
        this.request<UploadSession>(`/micro_apps/${microAppId}/upload_sessions`, {
          method: "POST",
          body: { upload: { filename: input.filename, byte_size: input.byteSize, content_type: input.contentType, checksum: input.checksum } },
          signal,
        }),
      put: (directUpload: DirectUpload, body: Uint8Array, signal?: AbortSignal) => this.upload(directUpload, body, signal),
    };

    this.publications = {
      list: (microAppId: number, signal?: AbortSignal) => this.request<Publication[]>(`/micro_apps/${microAppId}/micro_app_publications`, { signal }),
      get: (microAppId: number, publicationId: number, signal?: AbortSignal) =>
        this.request<Publication>(`/micro_apps/${microAppId}/micro_app_publications/${publicationId}`, { signal }),
      create: (microAppId: number, input: { signedBlobId: string; byteSize: number; sha256: string; manifest: MicroAppManifestV1 }, signal?: AbortSignal) =>
        this.request<Publication>(`/micro_apps/${microAppId}/micro_app_publications`, {
          method: "POST",
          body: { publication: { signed_blob_id: input.signedBlobId, byte_size: input.byteSize, sha256: input.sha256, manifest: input.manifest } },
          signal,
        }),
      finalize: (microAppId: number, publicationId: number, signal?: AbortSignal) =>
        this.request<Publication>(`/micro_apps/${microAppId}/micro_app_publications/${publicationId}/finalize`, { method: "POST", body: {}, signal }),
      configureEnvironment: (microAppId: number, publicationId: number, values: Record<string, string>, signal?: AbortSignal) =>
        this.request<Publication>(`/micro_apps/${microAppId}/micro_app_publications/${publicationId}/environment`, {
          method: "POST",
          body: { environment: values },
          signal,
        }),
      wait: (microAppId: number, publicationId: number, options: { timeoutMs?: number; intervalMs?: number; signal?: AbortSignal; onStatus?: (publication: Publication) => void } = {}) =>
        this.waitForPublication(microAppId, publicationId, options),
    };
  }

  async upload(directUpload: DirectUpload, body: Uint8Array, signal?: AbortSignal): Promise<void> {
    let response: Response;
    try {
      response = await this.fetchImpl(directUpload.url, {
        method: directUpload.method ?? "PUT",
        headers: directUpload.headers,
        body: Buffer.from(body),
        redirect: "error",
        ...(signal === undefined ? {} : { signal }),
      });
    } catch (error) {
      throw new NvgNetworkError(`Could not upload archive to storage: ${(error as Error).message}`, { cause: error });
    }
    if (!response.ok) throw new NvgNetworkError(`Storage upload failed with HTTP ${response.status}`);
  }

  async publish(options: PublishOptions): Promise<Publication> {
    const archive = await prepareArchive(options.source);
    try {
      const session = await this.uploads.createSession(options.microAppId, {
        filename: archive.filename,
        byteSize: archive.byteSize,
        contentType: "application/gzip",
        checksum: archive.md5,
      }, options.signal);
      await this.upload(session.direct_upload, await readFile(archive.path), options.signal);
      let publication = await this.publications.create(options.microAppId, {
        signedBlobId: session.signed_blob_id,
        byteSize: archive.byteSize,
        sha256: archive.sha256,
        manifest: options.manifest,
      }, options.signal);
      options.onStatus?.(publication);
      if (options.wait && !terminalStatuses.has(publication.status)) {
        publication = await this.waitForPublication(options.microAppId, publication.publication_id, {
          ...(options.timeoutMs === undefined ? {} : { timeoutMs: options.timeoutMs }),
          ...(options.intervalMs === undefined ? {} : { intervalMs: options.intervalMs }),
          ...(options.signal === undefined ? {} : { signal: options.signal }),
          ...(options.onStatus === undefined ? {} : { onStatus: options.onStatus }),
        });
      }
      return publication;
    } finally {
      await archive.cleanup();
    }
  }

  private async waitForPublication(
    microAppId: number,
    publicationId: number,
    options: { timeoutMs?: number; intervalMs?: number; signal?: AbortSignal; onStatus?: (publication: Publication) => void },
  ): Promise<Publication> {
    const deadline = Date.now() + (options.timeoutMs ?? 10 * 60_000);
    while (true) {
      const publication = await this.publications.get(microAppId, publicationId, options.signal);
      options.onStatus?.(publication);
      if (terminalStatuses.has(publication.status)) return publication;
      if (Date.now() >= deadline) throw new Error(`Timed out waiting for publication ${publicationId}`);
      await new Promise<void>((resolve, reject) => {
        if (options.signal?.aborted) {
          reject(options.signal.reason);
          return;
        }
        const onAbort = () => { clearTimeout(timer); reject(options.signal?.reason); };
        const timer = setTimeout(() => {
          options.signal?.removeEventListener("abort", onAbort);
          resolve();
        }, options.intervalMs ?? 2_000);
        options.signal?.addEventListener("abort", onAbort, { once: true });
      });
    }
  }

  private async request<T>(path: string, options: { method?: string; body?: unknown; signal?: AbortSignal | undefined } = {}): Promise<T> {
    let response: Response;
    try {
      response = await this.fetchImpl(`${this.baseUrl}/cli/v1${path}`, {
        method: options.method ?? "GET",
        headers: {
          Accept: "application/json",
          Authorization: `Bearer ${this.token}`,
          "Content-Type": "application/json",
          "User-Agent": this.userAgent,
        },
        ...(options.body === undefined ? {} : { body: JSON.stringify(options.body) }),
        ...(options.signal === undefined ? {} : { signal: options.signal }),
      });
    } catch (error) {
      throw new NvgNetworkError(`Could not reach ${this.baseUrl}: ${(error as Error).message}`, { cause: error });
    }

    let payload: unknown;
    try {
      payload = await response.json();
    } catch (error) {
      throw new NvgNetworkError(`Navegante returned a non-JSON response (HTTP ${response.status})`, { cause: error });
    }
    if (!response.ok) {
      const apiError = payload as Partial<ApiErrorBody>;
      throw new NvgApiError(
        response.status,
        apiError.error?.code ?? "unknown_error",
        apiError.error?.message ?? `Request failed with HTTP ${response.status}`,
        apiError.error?.details ?? {},
      );
    }
    if (typeof payload !== "object" || payload === null || !("data" in payload)) {
      throw new NvgNetworkError("Navegante response did not contain a data envelope");
    }
    return (payload as { data: T }).data;
  }
}

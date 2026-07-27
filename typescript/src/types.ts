export type AccessPolicy = "organization" | "private";
export type PublicationStatus = "draft" | "validating" | "needs_setup" | "deploying" | "published" | "failed";
export type ManifestMode = "static" | "build" | "service" | "container";

export interface VersionHistoryItem {
  publication_id: number;
  version: number;
  status: PublicationStatus;
  created_at: string;
  published_at: string | null;
}

export interface MicroAppSummary {
  id: number;
  micro_app_id: number;
  organization_id: number;
  name: string;
  slug: string;
  access_policy: AccessPolicy;
  current_publication_id: number | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface MicroApp extends MicroAppSummary {
  version_history: VersionHistoryItem[];
}

export interface MicroAppManifestV1 {
  version: 1;
  mode: ManifestMode;
  publish_dir?: string;
  project_root?: string;
  required_env?: string[];
  runtime?: "node" | "ruby" | "python" | "go" | "php" | "static" | "docker";
  node?: string | number;
  node_version?: string | number;
  build_command?: string;
  start_command?: string;
  app_port?: number;
  dockerfile?: string;
  build_context?: string;
  [key: string]: unknown;
}

export interface LogsSummary {
  available: boolean;
  excerpt: string | null;
}

export interface Publication {
  micro_app_id: number;
  publication_id: number;
  version: number;
  status: PublicationStatus;
  setup_url: string | null;
  management_url: string | null;
  live_url: string | null;
  health_status: string | null;
  deployment_status: string | null;
  deployment_id: number | null;
  missing_environment_names: string[];
  configured_environment_names?: string[];
  logs_summary: LogsSummary;
  version_history: VersionHistoryItem[];
  created_at: string;
  published_at: string | null;
}

export interface DirectUpload {
  url: string;
  headers: Record<string, string>;
  method?: string;
}

export interface UploadSession {
  signed_blob_id: string;
  direct_upload: DirectUpload;
  filename: string;
  byte_size: number;
  content_type: string;
  expires_at: string;
  cleanup: string;
}

export interface ApiErrorBody {
  error: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
  };
}

export interface PublishOptions {
  microAppId: number;
  source: string;
  manifest: MicroAppManifestV1;
  wait?: boolean;
  timeoutMs?: number;
  intervalMs?: number;
  signal?: AbortSignal;
  onStatus?: (publication: Publication) => void;
}

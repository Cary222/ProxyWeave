export interface Metadata {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  license?: string;
  repository?: string;
}

export interface HealthCheck {
  path: string;
  expected_status: number;
  busy_statuses?: number[];
  identity_header: string;
  identity_value: string;
  timeout_ms: number;
  startup_timeout_ms: number;
  max_retries: number;
}

export interface Configuration {
  required_secrets: string[];
  settings_schema?: Record<string, unknown>;
  default_settings?: Record<string, unknown>;
}

export interface Manifest {
  schema_version: string;
  metadata: Metadata;
  ownership: "managed" | "external";
  runtime?: unknown;
  configuration: Configuration;
  health_check: HealthCheck;
}

export interface PluginRecord {
  manifest: Manifest;
  package_path: string;
  data_dir: string;
  installed_at: string;
  updated_at: string;
  settings: Record<string, unknown>;
}

export interface ImportResult {
  record: PluginRecord;
  already_registered: boolean;
}

export interface PluginStatusInfo {
  status: "stopped" | "starting" | "ready" | "busy" | "failed";
  port?: number | null;
  pid?: number | null;
  reason?: string | null;
  error?: string | null;
  active_requests: number;
}

export interface SecretStatusInfo {
  name: string;
  is_set: boolean;
}

export interface GatewayStatusInfo {
  is_running: boolean;
  port: number;
  listen_address?: string | null;
  active_backend?: string | null;
  master_key_preview: string;
}

export interface HostEvent {
  id: number;
  timestamp_ms: number;
  category: string;
  action: string;
  target: string;
  success: boolean;
  summary: string;
}

export interface TestCallResult {
  success: boolean;
  status: number;
  latency_ms: number;
  response_body: string;
}

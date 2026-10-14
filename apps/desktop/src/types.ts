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

export interface RuntimeConfig {
  ownership: "managed" | "external";
  executable?: string;
  sha256?: string;
  endpoint?: string;
  [key: string]: unknown;
}

export interface Manifest {
  schema_version: string;
  metadata: Metadata;
  runtime: RuntimeConfig;
  ownership?: "managed" | "external"; // 保留可选兼容
  configuration: Configuration;
  capabilities?: { protocol: string; models_path: string; inference_path: string; streaming: boolean };
  management?: { path: string; presentation: "browser" } | null;
  health_check?: HealthCheck;
  [key: string]: unknown;
}

export interface PluginRecord {
  manifest: Manifest;
  package_path?: string;
  data_dir?: string;
  installed_at?: string;
  updated_at?: string;
  settings: Record<string, unknown>;
  fingerprint?: string;
  [key: string]: unknown;
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

// Optional on the wire for older hosts; absence means unknown, never inferred from running.
export interface GatewayProtocolCapabilities {
  ingress: { protocol: string; endpoints: string[] }[];
  backend_protocols: string[];
  ir_source: string;
}

export interface GatewayStatusInfo {
  is_running: boolean;
  port: number;
  listen_address?: string | null;
  active_backend?: string | null;
  master_key_preview: string;
  protocol_capabilities?: GatewayProtocolCapabilities | null;
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

export interface ManagementLink {
  state: "undeclared" | "unavailable" | "available";
  url: string | null;
}

export type ThemeMode = "sky" | "glass" | "dark";

export type NavTab =
  | "overview"
  | "gateway"
  | "plugins"
  | "discover"
  | "flight"
  | "audit"
  | "ide"
  | "settings";

// ---------------------------------------------------------------------------
// Flight Seam DTOs
// ---------------------------------------------------------------------------

export interface FlightDiscoveredModel {
  id: string;
  name: string;
  created?: number | null;
  owned_by?: string | null;
}

export interface FlightModelListResult {
  models: FlightDiscoveredModel[];
  active_backend?: string | null;
}

export interface FlightMessageInput {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface FlightChatRequestInput {
  request_id?: string | null;
  model: string;
  messages: FlightMessageInput[];
  temperature?: number | null;
  stream?: boolean | null;
}

export interface FlightChatStartResult {
  request_id: string;
  model: string;
  status?: string;
}

export interface FlightUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

export type FlightStreamEvent =
  | {
      type: "started";
      request_id: string;
      model: string;
    }
  | {
      type: "delta";
      request_id?: string;
      text?: string | null;
      delta?: string | null;
      reasoning?: string | null;
      reasoning_delta?: string | null;
      first_content_ms?: number | null;
      elapsed_ms?: number;
    }
  | {
      type: "completed";
      request_id: string;
      total_text: string;
      total_reasoning?: string | null;
      first_content_ms?: number | null;
      elapsed_ms: number;
      usage?: FlightUsage | null;
      finish_reason?: string | null;
      saw_done: boolean;
    }
  | {
      type: "done";
      request_id?: string;
      finish_reason?: string | null;
      usage?: FlightUsage | null;
      elapsed_ms: number;
      first_content_ms?: number | null;
    }
  | {
      type: "cancelled";
      request_id: string;
      elapsed_ms: number;
    }
  | {
      type: "failed";
      request_id: string;
      error_message: string;
      error?: string;
      code?: string | null;
      elapsed_ms: number;
    }
  | {
      type: "error";
      request_id?: string;
      error: string;
      code?: string | null;
      elapsed_ms: number;
    };

export interface FlightChatNonstreamResult {
  request_id: string;
  model: string;
  text: string;
  content?: string;
  reasoning?: string | null;
  first_content_ms?: number | null;
  elapsed_ms: number;
  usage?: FlightUsage | null;
  finish_reason?: string | null;
}

export interface FlightCancelResult {
  cancelled: boolean;
  message?: string;
}

// ---------------------------------------------------------------------------
// Agent / IDE Integration Seam DTOs
// ---------------------------------------------------------------------------

export interface AgentTargetStatus {
  id: string;
  label: string;
  protocol: string;
  supported: boolean;
  installed: boolean;
  configured: boolean;
  config_path: string;
  note: string;
  version?: string | null;
}

export interface AgentImportOutcome {
  target: string;
  backup_dir: string;
  files: string[];
  models: string[];
}

export interface AgentRestoreOutcome {
  target: string;
  backup_id: string;
  restored_files: number;
}

export interface AgentBackupItem {
  id: string;
  created_at: number;
  path: string;
}

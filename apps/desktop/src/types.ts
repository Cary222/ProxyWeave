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
  // Read-only display contract (manifest-declared); the host has no idle reaper today,
  // null disables idle reclaim; a numeric value is declared configuration, not active behavior.
  lifecycle?: {
    startup_timeout_ms?: number | null;
    idle_timeout_ms?: number | null;
    max_port_retries?: number;
  } | null;
  [key: string]: unknown;
}
export interface InstalledSourceDto {
  owner: string;
  repo: string;
  release_id: number;
  tag_name: string;
  asset_id: number;
  asset_name: string;
  target_os: string;
  target_arch: string;
  archive_sha256: string;
  expected_sha256: string;
  digest_evidence?: unknown;
  package_contract?: unknown;
  manifest_fingerprint?: string;
  immutability?: unknown;
  [key: string]: unknown;
}

export type PluginUpdateStatus =
  | "update_available"
  | "up_to_date"
  | "release_changed"
  | "installed_newer"
  | "not_checkable"
  | "forbidden"
  | "not_found"
  | "error";

export interface PluginUpdateStatusDto {
  status: PluginUpdateStatus | string;
  owned_update_allowed: boolean;
  latest_tag?: string | null;
  http_status?: number | null;
  rate_limit_remaining?: number | null;
  rate_limit_reset?: number | null;
  detail: string;
}

export type UpdateResultDto =
  | { status: "success"; old_version: string; new_version: string; restarted: boolean }
  | { status: "blocked_active_requests"; active_requests: number }
  | { status: "not_allowed_external" }
  | { status: "rollback_success"; error: string; rolled_back_to_version: string }
  | { status: "rollback_failed"; error: string; rollback_error: string };
export interface PluginRecord {
  manifest: Manifest;
  package_path?: string;
  data_dir?: string;
  installed_at?: string;
  updated_at?: string;
  settings: Record<string, unknown>;
  fingerprint?: string;
  source?: InstalledSourceDto | null;
  [key: string]: unknown;
}

export type PluginRecordDto = PluginRecord;

export interface ImportResult {
  record: PluginRecord;
  already_registered: boolean;
}

export interface DownloadProgressEvent {
  plugin_id: string;
  stage:
    | "fetching_release"
    | "downloading"
    | "verifying_checksum"
    | "extracting"
    | "registering"
    | "completed"
    | "error"
    | string;
  downloaded_bytes: number;
  total_bytes: number;
  percent: number;
  message: string;
}

export interface PluginStatusInfo {
  status: "stopped" | "starting" | "ready" | "busy" | "failed";
  port?: number | null;
  pid?: number | null;
  reason?: string | null;
  error?: string | null;
  active_requests: number;
  source?: InstalledSourceDto | null;
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
  route_strategy?: string | null;
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

export interface RequestTrace {
  id: string;
  timestamp_ms: number;
  client: string;
  client_ip: string;
  method: string;
  path: string;
  model?: string | null;
  ingress_protocol: string;
  route_strategy: string;
  target_plugin: string;
  status_code: number;
  duration_ms: number;
  first_token_ms?: number | null;
  input_tokens?: number | null;
  output_tokens?: number | null;
  cache_hit?: boolean | null;
  fallback_trail: string[];
  error?: string | null;
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

export interface FlightContentPart {
  type: string;
  text?: string | null;
  image_url?: { url: string } | null;
}

export interface FlightMessageInput {
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  content_parts?: FlightContentPart[] | null;
}

export interface FlightChatRequestInput {
  request_id?: string | null;
  model: string;
  messages: FlightMessageInput[];
  temperature?: number | null;
  stream?: boolean | null;
  tools?: unknown[] | null;
  tool_choice?: unknown | null;
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

export interface FlightToolCallFragment {
  index: number;
  id?: string | null;
  name?: string | null;
  arguments_delta?: string | null;
}

export interface FlightToolCall {
  id: string;
  type: string;
  name: string;
  arguments: string;
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
      type: "tool_call_delta";
      request_id: string;
      fragments: FlightToolCallFragment[];
      elapsed_ms: number;
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
      total_tool_calls?: FlightToolCall[] | null;
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
  tool_calls?: FlightToolCall[] | null;
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

export interface AgentFileDiff {
  path: string;
  existing_content?: string | null;
  proposed_content: string;
  original?: string | null;
  updated?: string;
  is_new?: boolean;
}

export interface AgentDiffPreview {
  target: string;
  diffs: AgentFileDiff[];
  models: string[];
  files?: AgentFileDiff[];
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

// ---------------------------------------------------------------------------
// Marketplace Seam DTOs
// ---------------------------------------------------------------------------

export interface MarketplacePluginItem {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  repo_url: string;
  kind: "managed" | "external" | string;
  category: "proxy" | "protocol" | "local_model" | "tools" | string;
  tags: string[];
  protocol: "openai_chat" | "openai_responses" | "anthropic_messages" | "multi" | string;
  stars?: number | null;
  download_url?: string | null;
  release_tag?: string | null;
  icon?: string | null;
  verified: boolean;
}

export interface MarketplaceCatalogResponse {
  items: MarketplacePluginItem[];
  is_offline: boolean;
  notice?: string | null;
  total: number;
}

import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke, Channel } from "./lib/tauri";
import {
  AgentBackupItem,
  AgentImportOutcome,
  AgentRestoreOutcome,
  AgentTargetStatus,
  FlightCancelResult,
  FlightChatNonstreamResult,
  FlightChatRequestInput,
  FlightChatStartResult,
  FlightModelListResult,
  FlightStreamEvent,
  GatewayStatusInfo,
  HostEvent,
  ImportResult,
  ManagementLink,
  PluginRecord,
  PluginStatusInfo,
  SecretStatusInfo,
  TestCallResult,
} from "./types";

interface TauriInternalsShape {
  invoke?: unknown;
  metadata?: {
    currentWindow?: {
      label?: unknown;
    };
  };
}

function resolveCurrentWindow() {
  try {
    if ("__TAURI_INTERNALS__" in globalThis) {
      const internals = (
        globalThis as { __TAURI_INTERNALS__?: TauriInternalsShape }
      ).__TAURI_INTERNALS__;
      if (internals?.invoke && internals.metadata?.currentWindow?.label) {
        return getCurrentWindow();
      }
    }
  } catch {
    // Fallback to invoke seam when Tauri window metadata is unavailable
  }
  return null;
}

export const api = {
  // Gateway IPC
  getGatewayStatus: () => invoke<GatewayStatusInfo>("get_gateway_status"),
  startGateway: () => invoke<GatewayStatusInfo>("start_gateway"),
  stopGateway: () => invoke<GatewayStatusInfo>("stop_gateway"),
  setGatewayPort: (port: number) => invoke<GatewayStatusInfo>("set_gateway_port", { port }),
  getGatewayMasterKey: () => invoke<string>("get_gateway_master_key"),
  regenerateGatewayMasterKey: () => invoke<string>("regenerate_gateway_master_key"),
  testGatewayModels: () => invoke<TestCallResult>("test_gateway_models"),
  getHostEvents: (limit?: number) => invoke<HostEvent[]>("get_host_events", { limit }),

  // Plugins IPC
  listPlugins: () => invoke<PluginRecord[]>("list_local_plugins"),
  listLocalPlugins: () => invoke<PluginRecord[]>("list_local_plugins"),
  getPluginRuntimeStatus: (id: string) =>
    invoke<PluginStatusInfo>("get_plugin_runtime_status", { id }),
  getPluginSecretsStatus: (id: string) =>
    invoke<SecretStatusInfo[]>("get_plugin_secrets_status", { id }),
  getPluginApiKey: (id: string) =>
    invoke<string | null>("get_plugin_api_key", { id }),
  savePluginSecret: (id: string, name: string, value: string) =>
    invoke<void>("save_plugin_secret", { id, name, value }),
  deletePluginSecret: (id: string, name: string) =>
    invoke<void>("delete_plugin_secret", { id, name }),
  startPlugin: (id: string) =>
    invoke<PluginStatusInfo>("start_plugin", { id }),
  stopPlugin: (id: string) =>
    invoke<PluginStatusInfo>("stop_plugin", { id }),
  restartPlugin: (id: string) =>
    invoke<PluginStatusInfo>("restart_plugin", { id }),
  selectPlugin: (id: string) =>
    invoke<void>("select_plugin", { id }),
  savePluginSettings: (id: string, settings: Record<string, unknown>) =>
    invoke<void>("save_plugin_settings", { id, settings }),
  importLocalPlugin: (manifestPath: string) =>
    invoke<ImportResult>("import_local_plugin", { manifestPath }),
  importProjectFromDirectory: (path: string) =>
    invoke<ImportResult>("import_project_from_directory", { path }),
  importProjectFromGithub: (url: string) =>
    invoke<ImportResult>("import_project_from_github", { url }),
  getPluginManagement: (id: string) =>
    invoke<ManagementLink>("get_plugin_management", { id }),
  openPluginManagement: (id: string) =>
    invoke<void>("open_plugin_management", { id }),

  // Flight IPC
  flightListModels: () => invoke<FlightModelListResult>("flight_list_models"),
  flightChatStream: (
    input: FlightChatRequestInput,
    onEvent: (event: FlightStreamEvent) => void,
  ) => {
    const channel = new Channel<FlightStreamEvent>();
    channel.onmessage = onEvent;
    return invoke<FlightChatStartResult>("flight_chat_stream", { channel, input });
  },
  flightChatNonstream: (input: FlightChatRequestInput) =>
    invoke<FlightChatNonstreamResult>("flight_chat_nonstream", { input }),
  flightCancel: (requestId: string) =>
    invoke<FlightCancelResult>("flight_cancel", { requestId }),

  // Agent / IDE Integration IPC
  detectAgentClients: () =>
    invoke<AgentTargetStatus[]>("detect_agent_clients"),
  importAgentClient: (target: string, models: string[]) =>
    invoke<AgentImportOutcome>("import_agent_client", { target, models }),
  restoreAgentClient: (target: string, backupId?: string | null) =>
    invoke<AgentRestoreOutcome>("restore_agent_client", { target, backupId }),
  listAgentBackups: (target: string) =>
    invoke<AgentBackupItem[]>("list_agent_backups", { target }),

  // App window controls (DEF-018: real Tauri window actions, no no-op / no swallowed errors)
  closeWindow: () => invoke<void>("quit_app"),
  minimizeWindow: () => {
    const win = resolveCurrentWindow();
    if (win) {
      return win.minimize();
    }
    return invoke<void>("plugin:window|minimize", { label: "main" });
  },
  toggleMaximizeWindow: () => {
    const win = resolveCurrentWindow();
    if (win) {
      return win.toggleMaximize();
    }
    return invoke<void>("plugin:window|toggle_maximize", { label: "main" });
  },
  maximizeWindow: () => {
    const win = resolveCurrentWindow();
    if (win) {
      return win.toggleMaximize();
    }
    return invoke<void>("plugin:window|toggle_maximize", { label: "main" });
  },
  hideWindow: () => {
    const win = resolveCurrentWindow();
    if (win) {
      return win.hide();
    }
    return invoke<void>("plugin:window|hide", { label: "main" });
  },
  quitApp: () => invoke<void>("quit_app"),
  showMenubar: () => invoke<void>("show_menubar"),
};

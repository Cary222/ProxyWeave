import { api } from "../../api";
import type { AgentTargetStatus, FlightDiscoveredModel } from "../../types";

export type AgentClientTarget = AgentTargetStatus & { configPath: string };
export interface AgentDetectionResult { targets: AgentClientTarget[] }
export type GatewayModelItem = FlightDiscoveredModel;
export interface AgentBackupItem { id: string; createdAt: number; path: string }

// Route availability is not native-client acceptance. Keep unverified mappings blocked.
export function canWriteTarget(target: AgentClientTarget): boolean {
  if (!target.installed || !target.supported) return false;
  if (target.id === "claude-code") return target.protocol === "Anthropic Messages";
  return target.protocol === "OpenAI Chat"
    && !["claude-desktop", "codex", "grok-build"].includes(target.id);
}

// Capabilities describe compiled routes/backends, not config writers or native acceptance.
export function gatewayCapabilities(status: import("../../types").GatewayStatusInfo | null) {
  const caps = status?.protocol_capabilities;
  if (!caps || !Array.isArray(caps.ingress) || !Array.isArray(caps.backend_protocols)
      || typeof caps.ir_source !== "string" || !caps.ir_source.trim()
      || !caps.ingress.every(item => item && typeof item.protocol === "string"
        && Array.isArray(item.endpoints) && item.endpoints.every(path => typeof path === "string"))
      || !caps.backend_protocols.every(protocol => typeof protocol === "string")) return null;
  return caps;
}
export function clientIngressProtocol(target: AgentClientTarget): string | null {
  if (target.protocol === "OpenAI Chat") return "openai_chat";
  if (["Anthropic Messages", "Anthropic Messages (3P)"].includes(target.protocol)) return "anthropic_messages";
  if (target.protocol === "OpenAI Responses") return "openai_responses";
  return null;
}
export function canRouteTarget(target: AgentClientTarget, status: import("../../types").GatewayStatusInfo | null): boolean {
  const caps = gatewayCapabilities(status);
  const protocol = clientIngressProtocol(target);
  return protocol !== null && caps?.ingress.some(item => item.protocol === protocol && item.endpoints.length > 0) === true;
}
export async function detectAgentClients(): Promise<AgentDetectionResult> {
  const targets = await api.detectAgentClients();
  return { targets: targets.map(target => ({ ...target, configPath: target.config_path })) };
}
export async function getGatewayModels(): Promise<GatewayModelItem[]> {
  return (await api.flightListModels()).models;
}
export const importAgentClient = api.importAgentClient;
export async function listAgentBackups(target: string): Promise<{ backups: AgentBackupItem[] }> {
  return { backups: (await api.listAgentBackups(target)).map(item => ({
    id: item.id, createdAt: item.created_at, path: item.path,
  })) };
}
export async function restoreAgentClient(target: string, backupId?: string) {
  const result = await api.restoreAgentClient(target, backupId);
  return { ...result, restored: result.restored_files };
}

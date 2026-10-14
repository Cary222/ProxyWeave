// Thin host entry; the workbench is directly extracted from WorkBuddy AgentsPage.
import AgentsPage from "./ide-source/AgentsPage";
import type { GatewayStatusInfo } from "../types";

export interface IdePanelProps {
  gatewayStatus: GatewayStatusInfo | null;
  masterKey?: string | null;
  onNavigateGateway?: () => void;
}

export function IdePanel(props: IdePanelProps) {
  return <AgentsPage {...props} />;
}

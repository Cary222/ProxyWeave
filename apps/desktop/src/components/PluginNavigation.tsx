import { useEffect, useRef, useState } from "react";
import { PluginRecord, PluginStatusInfo } from "../types";
import { Icon } from "./Icon";

export function reconcilePluginSelection(id: string | null, plugins: PluginRecord[]): string | null {
  return id !== null && plugins.some((p) => p.manifest.metadata.id === id) ? id : null;
}

interface Props {
  dataReady?: boolean;
  plugins: PluginRecord[];
  statuses: Record<string, PluginStatusInfo>;
  selectedId: string | null;
  active: boolean;
  onSelect: (id: string | null) => void;
  onOpen: () => void;
}

export function PluginNavigation({ dataReady = true, plugins, statuses, selectedId, active, onSelect, onOpen }: Props) {
  const [expanded, setExpanded] = useState(false);
  const group = useRef<HTMLButtonElement>(null);
  const overview = useRef<HTMLButtonElement>(null);
  const focusedId = useRef<string | null>(null);
  useEffect(() => {
    const id = focusedId.current;
    if (id && !plugins.some((p) => p.manifest.metadata.id === id)) {
      focusedId.current = null;
      if (document.activeElement === document.body) (expanded ? overview : group).current?.focus();
    }
  }, [plugins, expanded]);
  const running = Object.values(statuses).filter((s) => s.status === "ready" || s.status === "busy").length;
  return <div className="plugin-navigation" onBlur={(event) => {
    if (event.relatedTarget && !event.currentTarget.contains(event.relatedTarget as Node)) focusedId.current = null;
  }}>
    <button type="button" ref={group} className={active ? "active" : ""}
      aria-expanded={expanded} aria-controls="plugin-navigation-items" onClick={() => {
        group.current?.focus();
        setExpanded(!expanded);
        onOpen();
      }}>
      <Icon name="plug"/>
      <span className="nav-label">插件管理</span>
      <span className="nav-badge">{!dataReady ? "未知" : plugins.every(p => statuses[p.manifest.metadata.id]) ? `${running}/${plugins.length}` : `未知/${plugins.length}`}</span>
    </button>
    <div id="plugin-navigation-items" hidden={!expanded}>
      <button type="button" ref={overview} className={active && selectedId === null ? "active" : ""}
        aria-current={active && selectedId === null ? "page" : undefined}
        onFocus={() => { focusedId.current = null; }} onClick={() => onSelect(null)}>总览 / 导入</button>
      {plugins.map(({ manifest: { metadata } }) => <button type="button" key={metadata.id}
        className={active && selectedId === metadata.id ? "active" : ""}
        aria-current={active && selectedId === metadata.id ? "page" : undefined}
        onFocus={() => { focusedId.current = metadata.id; }} onClick={() => onSelect(metadata.id)}>
        <span className="plugin-nav-name">{metadata.name}<small>{metadata.id}</small></span>
        <small>{statuses[metadata.id]?.status ?? "未知"}</small>
      </button>)}
    </div>
  </div>;
}

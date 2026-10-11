import React from "react";
import { HostEvent } from "../types";

interface EventsPanelProps {
  events: HostEvent[];
  onRefresh: () => void;
}

export const EventsPanel: React.FC<EventsPanelProps> = ({ events, onRefresh }) => {
  return (
    <div className="panel-section">
      <div className="section-header">
        <div>
          <h3 className="section-title">宿主审计事件 (Host Event Trail)</h3>
          <p className="section-desc">
            记录宿主对插件导入、启停控制、凭据落库与网关生命周期的操作轨迹（最近 50 条）。
          </p>
        </div>
        <button type="button" className="secondary-action" onClick={onRefresh}>
          刷新记录
        </button>
      </div>

      <div className="events-container card-box">
        {events.length === 0 ? (
          <div className="empty-state">
            <p>暂无宿主审计事件记录。</p>
          </div>
        ) : (
          <div className="events-table-wrapper">
            <table className="events-table">
              <thead>
                <tr>
                  <th>时间</th>
                  <th>类别</th>
                  <th>动作</th>
                  <th>对象</th>
                  <th>结果</th>
                  <th>摘要说明</th>
                </tr>
              </thead>
              <tbody>
                {events
                  .slice()
                  .reverse()
                  .map((ev) => {
                    const dateStr = new Date(ev.timestamp_ms).toLocaleTimeString();
                    return (
                      <tr key={ev.id} className={ev.success ? "tr-success" : "tr-failed"}>
                        <td className="text-muted font-mono">{dateStr}</td>
                        <td>
                          <span className="event-category-badge">{ev.category}</span>
                        </td>
                        <td>
                          <code>{ev.action}</code>
                        </td>
                        <td>
                          <span className="font-mono">{ev.target}</span>
                        </td>
                        <td>
                          <span
                            className={`status-pill ${
                              ev.success ? "status-pill--ready" : "status-pill--failed"
                            }`}
                          >
                            {ev.success ? "SUCCESS" : "FAILED"}
                          </span>
                        </td>
                        <td className="event-summary-cell">{ev.summary}</td>
                      </tr>
                    );
                  })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
};

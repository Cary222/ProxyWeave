import { CSSProperties, ReactNode } from "react";

// Directly reused SVG symbols from docs/design/proxyweave.html
const paths: Record<string, ReactNode> = {
  grid: <><rect x="3.5" y="3.5" width="7" height="7" rx="2"/><rect x="13.5" y="3.5" width="7" height="7" rx="2"/><rect x="3.5" y="13.5" width="7" height="7" rx="2"/><rect x="13.5" y="13.5" width="7" height="7" rx="2"/></>,
  globe: <><circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17M12 3.5c2.4 2.3 3.6 5.1 3.6 8.5s-1.2 6.2-3.6 8.5c-2.4-2.3-3.6-5.1-3.6-8.5S9.6 5.8 12 3.5z"/></>,
  plug: <><path d="M9 3.5V8M15 3.5V8M6.5 8h11V12a5.5 5.5 0 0 1-11 0V8zM12 17.5v3"/></>,
  compass: <><circle cx="12" cy="12" r="8.5"/><path d="M15.5 8.5l-2 5-5 2 2-5z"/></>,
  send: <><path d="M21 3L10.8 13.2M21 3l-6.8 18-3.4-7.8L3 9.8z"/></>,
  pulse: <><path d="M3 12h4l2.5-6.5 4 12L16 12h5"/></>,
  term: <><rect x="3" y="4.5" width="18" height="15" rx="2.5"/><path d="M7 9.5l3.5 3L7 15.5M13 15.5h4.5"/></>,
  sliders: <><path d="M4 7.5h9M17.5 7.5H20M4 16.5h3.5M12 16.5h8"/><circle cx="15" cy="7.5" r="2.2"/><circle cx="9.5" cy="16.5" r="2.2"/></>,
  play: <><path d="M8 5.5v13l11-6.5z"/></>,
  pause: <><path d="M8.5 5.5v13M15.5 5.5v13"/></>,
  stop: <><rect x="6.5" y="6.5" width="11" height="11" rx="2"/></>,
  restart: <><path d="M20 12a8 8 0 1 1-2.3-5.6M20 3.5V8h-4.5"/></>,
  back: <><path d="M19.5 12h-15M10 6l-6 6 6 6"/></>,
  ext: <><path d="M14 4.5h5.5V10M19.5 4.5L11 13M9 5H7a2.5 2.5 0 0 0-2.5 2.5v9A2.5 2.5 0 0 0 7 19h9a2.5 2.5 0 0 0 2.5-2.5V14"/></>,
  check: <><path d="M4.5 12.5l5 5L19.5 7"/></>,
  key: <><circle cx="8" cy="14.5" r="4.5"/><path d="M11.2 11.3L20 2.5M17 5.5l3 3M14 8.5l2.5 2.5"/></>,
  copy: <><rect x="9" y="9" width="11" height="11" rx="2.5"/><path d="M5.5 15H5a1.5 1.5 0 0 1-1.5-1.5v-8A1.5 1.5 0 0 1 5 4h8A1.5 1.5 0 0 1 14.5 5.5V6"/></>,
  eye: <><path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="3"/></>,
  shield: <><path d="M12 3l7 2.8v5.4c0 4.4-3 7.6-7 9.3-4-1.7-7-4.9-7-9.3V5.8z"/><path d="M9 11.8l2.2 2.2L15.5 9.5"/></>,
  doc: <><path d="M6 3.5h8L19 8.5v11a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 5 19.5v-15A1.5 1.5 0 0 1 6 3.5z"/><path d="M13.5 3.5v5.5H19M8.5 13h7M8.5 16.5h7"/></>,
  gear: <><circle cx="12" cy="12" r="3.2"/><path d="M12 3.5v2.6M12 17.9v2.6M3.5 12h2.6M17.9 12h2.6M6 6l1.8 1.8M16.2 16.2L18 18M18 6l-1.8 1.8M7.8 16.2L6 18"/></>,
  layers: <><path d="M12 3.5l9 5-9 5-9-5z"/><path d="M3.5 13.5L12 18l8.5-4.5"/></>,
};

export type IconName = keyof typeof paths;

interface IconProps {
  name: IconName;
  className?: string;
  style?: CSSProperties;
}

export function Icon({ name, className = "", style }: IconProps) {
  const cls = className ? `ic ${className}` : "ic";
  return (
    <svg className={cls} style={style} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      {paths[name]}
    </svg>
  );
}

import "./App.css";
import "./menubar.css";
import { invoke } from "./lib/tauri";
export default function MenuBarPanel() {
  return <main className="floating-window"><h2>ProxyWeave</h2><p>Desktop baseline · no providers running</p><button onClick={() => void invoke("show_main_window")}>Open ProxyWeave</button><button onClick={() => void invoke("quit_app")}>Quit ProxyWeave</button></main>;
}

import React from "react";
import ReactDOM from "react-dom/client";

// Fonts bundled locally (no runtime CDN — hard rule 1). Anton = display; Jost = body.
import "@fontsource/anton/400.css";
import "@fontsource/jost/400.css";
import "@fontsource/jost/500.css";
import "@fontsource/jost/600.css";
import "@fontsource/jost/700.css";

import "./styles/tokens.css";
import "./index.css";
import App from "./App";
import { Glance } from "./components/glance/Glance";

// The menubar tray opens a separate webview window at `#/glance` (see src-tauri tray setup).
// Same bundle, different root — the popover, not the full app.
const isGlance = window.location.hash === "#/glance";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{isGlance ? <Glance /> : <App />}</React.StrictMode>,
);

import React from "react";
import ReactDOM from "react-dom/client";
import { isTauri } from "@tauri-apps/api/core";
import "@fontsource/jetbrains-mono/latin-500.css";
import "@fontsource/jetbrains-mono/latin-600.css";
import App from "./App";
import { LanguageProvider } from "./i18n";

// Only the direct-distribution macOS Tauri window supplies native vibrancy.
// Browser previews (including on a Mac) and other hosts need an opaque canvas.
document.documentElement.dataset.windowMaterial =
  isTauri() && navigator.platform.startsWith("Mac") ? "native-glass" : "opaque";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <LanguageProvider>
      <App />
    </LanguageProvider>
  </React.StrictMode>,
);

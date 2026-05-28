import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { settings } from "./api/client";
import "./styles.css";

const themes = ["warm", "cool", "bold"];

async function boot() {
  try {
    const current = await settings.get();
    if (themes.includes(current.theme)) {
      document.documentElement.dataset.theme = current.theme;
    }
  } catch {
    // Keep the CSS default warm theme.
  }

  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}

void boot();

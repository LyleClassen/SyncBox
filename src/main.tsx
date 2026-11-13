import { ToastViewport } from "@astryxdesign/core/Toast";
import { Theme } from "@astryxdesign/core/theme";
import { gothicTheme } from "@astryxdesign/theme-gothic/built";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Theme theme={gothicTheme} mode="dark">
      <ToastViewport position="bottomEnd">
        <App />
      </ToastViewport>
    </Theme>
  </React.StrictMode>,
);

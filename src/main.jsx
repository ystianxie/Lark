import "./diagnostics-bootstrap.js";
import {reactErrorHandlers} from "./diagnostics.js";
import React from "react";
import ReactDOM from "react-dom/client";
import ThemeProvider from "./theme/ThemeProvider";
import App from "./App";
import NotificationWindow from "./notification/NotificationWindow";

const isNotificationWindow = new URLSearchParams(window.location.search).get("window") === "notification";
ReactDOM.createRoot(document.getElementById("root"), reactErrorHandlers).render(
  <React.StrictMode>
    <ThemeProvider>
      {isNotificationWindow ? <NotificationWindow /> : <App />}
    </ThemeProvider>
  </React.StrictMode>
);


import React from "react";
import ReactDOM from "react-dom/client";
import ThemeProvider from "./theme/ThemeProvider";
import App from "./App";
import NotificationWindow from "./notification/NotificationWindow";

const isNotificationWindow = new URLSearchParams(window.location.search).get("window") === "notification";
ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <ThemeProvider>
      {isNotificationWindow ? <NotificationWindow /> : <App />}
    </ThemeProvider>
  </React.StrictMode>
);


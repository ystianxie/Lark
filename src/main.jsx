import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import NotificationWindow from "./notification/NotificationWindow";

const isNotificationWindow = new URLSearchParams(window.location.search).get("window") === "notification";
ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
      {isNotificationWindow ? <NotificationWindow /> : <App />}
  </React.StrictMode>
);

import React from "react";
import ReactDOM from "react-dom/client";
import { ConfigProvider } from "antd";
import App from "./App";
import NotificationWindow from "./notification/NotificationWindow";

const isNotificationWindow = new URLSearchParams(window.location.search).get("window") === "notification";
ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    {isNotificationWindow ? (
      <NotificationWindow />
    ) : (
      <ConfigProvider
        theme={{
          token: {
            fontFamily: "var(--font-family-ui)",
          },
        }}
      >
        <App />
      </ConfigProvider>
    )}
  </React.StrictMode>
);


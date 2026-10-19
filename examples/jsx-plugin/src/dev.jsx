import { previewPlugin } from "@lark/plugin-sdk/dev";
import App from "./App.jsx";

const runtime = previewPlugin(App, document.getElementById("root"));
if (import.meta.hot) import.meta.hot.dispose(() => runtime.unmount());

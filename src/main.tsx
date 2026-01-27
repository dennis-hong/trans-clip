import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { PostItEditorWindow } from "./components/DrawerPanel";
import "./styles.css";

// Check if this is an editor window
const params = new URLSearchParams(window.location.search);
const isEditorWindow = params.get("window") === "editor";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isEditorWindow ? <PostItEditorWindow /> : <App />}
  </React.StrictMode>
);

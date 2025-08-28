import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./ui/App";

const elem = document.getElementById("root")!;
const app = (
  <StrictMode>
    <App />
  </StrictMode>
);

if (import.meta.hot) {
  // with hot module reloading, import.meta.hot.data is persisted
  const root = (import.meta.hot.data as any).root ??= createRoot(elem);
  root.render(app);
} else {
  createRoot(elem).render(app);
}

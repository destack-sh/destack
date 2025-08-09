// preload first
import "./preload";

// regular imports
import { ENV, IS_DEV } from "@destack/utils/env";
import "./assets/index.css";

async function init() {
  console.group(`%csystem`, "color:yellow");
  console.info(
    `%cENV: ${ENV ?? "<unknown>"} (${IS_DEV ? "DEV MODE" : "PROD MODE"})`,
    "color:yellow",
  );
  // minimal app: no framework, no graph
  console.groupEnd();

  // prevent opening files that are dragged over the window
  window.addEventListener("dragover", (e) => e.preventDefault(), false);
  window.addEventListener("drop", (e) => e.preventDefault(), false);

  const app = document.getElementById("app");
  if (!app) throw new Error("app element not found");

  const container = document.createElement("div");
  container.style.width = "100vw";
  container.style.height = "100vh";
  container.style.display = "flex";
  container.style.alignItems = "center";
  container.style.justifyContent = "center";
  container.style.fontFamily = "system-ui, -apple-system, Segoe UI, Roboto, sans-serif";
  container.textContent = `Destack minimal build (${IS_DEV ? "dev" : "prod"})`;
  app.append(container);
}

init();



// preload first
import "./preload";

// regular imports
import { VERSION } from "@destack/language";
import { ENV, IS_DEV } from "@destack/utils/env";
import { getLogger } from "@destack/utils/log";
import { trace } from "@opentelemetry/api";
import { render } from "preact";
import "./assets/index.css";
import Destack from "./Destack";

const logger = getLogger("main");
const tracer = trace.getTracer("destack-ts-web");

async function init() {
  // dump startup info
  console.group(`%csystem`, "color:yellow");
  console.info(
    `%cENV: ${ENV ?? "<unknown>"} (${IS_DEV ? "DEV MODE" : "PROD MODE"})`,
    "color:yellow",
  );
  console.info(`%cVERSION: ${VERSION}`, "color:yellow");
  console.groupEnd();
  logger.info("web.start");

  // prevent opening files that are dragged over the window
  window.addEventListener("dragover", (e) => e.preventDefault(), false);
  window.addEventListener("drop", (e) => e.preventDefault(), false);

  render(<Destack />, document.getElementById("app")!);
}

init();

/* eslint-disable no-console */
import "./polyfills";

import { VERSION } from "@destack/language";
import { ENV, IS_DEV, TELEMETRY } from "@destack/utils/env";
import { getLogger } from "@destack/utils/log";
import { trace } from "@opentelemetry/api";
import posthog from "posthog-js";
import React from "react";
import ReactDOM from "react-dom/client";
import "./assets/index.css";
import Destack from "./Destack";

const logger = getLogger("main");
const tracer = trace.getTracer("destack-ts-web");

async function init() {
  // telemetry
  if (TELEMETRY) {
    posthog.init("phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma", {
      api_host: "https://e.destack.dev",
      ui_host: "https://eu.posthog.com",
      enable_recording_console_log: true,
      autocapture: true,
      session_recording: {
        maskAllInputs: false,
        maskInputOptions: {
          password: true,
          email: false,
          text: false,
          number: false,
          tel: false,
        },
      },
    });
    posthog.opt_in_capturing();
  } else {
    posthog.opt_out_capturing();
  }

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

  const root = ReactDOM.createRoot(document.getElementById("app")!);
  root.render(
    <React.StrictMode>
      <Destack />
    </React.StrictMode>,
  );
}

init();

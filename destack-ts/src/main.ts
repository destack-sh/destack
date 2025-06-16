/* eslint-disable no-console */
import "./assets/index.css";

import { keytrap } from "@/ui/keymap";
import { COMMIT, ENV, IS_DEV, SUPERVISOR_URL, TELEMETRY, VERSION } from "@/utils/globals";
import { onUnhandledError } from "@/utils/telemetry";
import posthog from "posthog-js";
import { createApp } from "vue";
import Space from "./Space.vue";

async function init() {
  const app = createApp(Space);

  // telemetry
  if (TELEMETRY) {
    posthog.init("phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma", {
      api_host: "https://e.destack.app",
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
  console.info(`%cENV: ${ENV ?? "<unknown>"} (${IS_DEV ? "DEV MODE" : "PROD MODE"})`, "color:yellow");
  console.info(`%cVERSION: ${VERSION}`, "color:yellow");
  console.info(`%cCOMMIT: ${COMMIT ?? "<unknown>"}`, "color:yellow");
  console.info(`%cSUPERVISOR_URL: ${SUPERVISOR_URL}`, "color:yellow");
  console.groupEnd();

  // prevent opening files that are dragged over the window
  window.addEventListener("dragover", (e) => e.preventDefault(), false);
  window.addEventListener("drop", (e) => e.preventDefault(), false);

  // setup vue stuff
  app.config.errorHandler = (err, instance, info) => onUnhandledError(err);
  window.onerror = (err) => onUnhandledError(err);

  // setup our own stuff
  keytrap.track(document);

  app.mount("#app");
}

init();

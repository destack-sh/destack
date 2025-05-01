/* eslint-disable no-console */
import "./assets/index.css";
import "./assets/prosemirror.css";

import {
  resetTransactionBuffers,
  startTransactionRotation as startTransactionBuffers,
} from "@/language/core/transaction";
import { RESOURCE_COMMANDS } from "@/language/resource/resource";
import { sendRemoteKeepAlives } from "@/system/connection";
import { DEBUG_COMMANDS } from "@/system/debug";
import { HISTORY_COMMANDS } from "@/system/edit";
import { watchCommands } from "@/ui/command";
import { keytrap } from "@/ui/keymap";
import { HOVER_MENU_DIRECTIVE, MENU_DIRECTIVE } from "@/ui/popover";
import { toaster } from "@/ui/toast";
import { EVENT_OUTSIDE_DIRECTIVE, HOVER_DIRECTIVE, TOOLTIP_DIRECTIVE } from "@/ui/tooltip";
import { COMMIT, ENV, GRPC_KEEPALIVE_INTERVAL_SECONDS, IS_DEV, SUPERVISOR_URL, VERSION } from "@/utils/globals";
import { log } from "@/utils/log";
import { registerViewComponents } from "@/views/registry";
import "highlight.js/styles/github.min.css";
import posthog from "posthog-js";
import { createApp } from "vue";
import Space from "./Space.vue";

function onUnhandledError(err: unknown) {
  if (typeof err === "string" && err.includes("ResizeObserver")) {
    return; // TODO :Robustness: don't just suppress ResizeObserver errors
  }
  log.error("error.internal", err);
  toaster.error({ title: "Internal client error", text: (err as any).message });
  captureException(err);
}

/** Handle top-level errors. */
function captureException(err: unknown) {
  posthog.captureException(err);
  resetTransactionBuffers();
}

async function init() {
  const app = createApp(Space);

  // telemetry
  if (!IS_DEV) {
    posthog.init("phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma", {
      api_host: "https://e.heybench.com",
      ui_host: "https://eu.posthog.com",
      enable_recording_console_log: true,
      autocapture: true,
    });
    posthog.opt_in_capturing();
  } else {
    posthog.opt_out_capturing();
  }

  // dump startup info
  console.group(`%cBench Web`, "color:orangered");
  console.info(`%cVersion: ${VERSION} (${COMMIT ?? "local"})`, "color:orangered");
  console.info(`%cEnvironment: ${ENV ?? "dev"}`, "color:orangered");
  console.info(`%cSupervisor: ${SUPERVISOR_URL}`, "color:orangered");
  console.groupEnd();

  // prevent opening files that are dragged over the window
  window.addEventListener("dragover", (e) => e.preventDefault(), false);
  window.addEventListener("drop", (e) => e.preventDefault(), false);

  // setup vue stuff
  app.config.errorHandler = (err, instance, info) => onUnhandledError(err);
  window.onerror = (err) => onUnhandledError(err);
  app.directive("tooltip", TOOLTIP_DIRECTIVE);
  app.directive("menu", MENU_DIRECTIVE);
  app.directive("hovermenu", HOVER_MENU_DIRECTIVE);
  app.directive("hover", HOVER_DIRECTIVE);
  app.directive("outside", EVENT_OUTSIDE_DIRECTIVE);

  // setup our own stuff
  await registerViewComponents();
  toaster.run();
  keytrap.track(document);
  startTransactionBuffers();
  setInterval(sendRemoteKeepAlives, GRPC_KEEPALIVE_INTERVAL_SECONDS * 1000);
  watchCommands();
  // (register commands that might not be imported directly)
  // eslint-disable-next-line @typescript-eslint/no-unused-expressions
  [HISTORY_COMMANDS, DEBUG_COMMANDS, RESOURCE_COMMANDS];

  app.mount("#app");
}

init();

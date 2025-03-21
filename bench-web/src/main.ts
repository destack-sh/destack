/* eslint-disable no-console */
import "./assets/index.css";

import { startTransactionRotation as startTransactionBuffers } from "@/language/runtime/transaction";
import { sendRemoteKeepAlives } from "@/system/connection";
import { keytrap } from "@/ui/keymap";
import { HOVER_MENU_DIRECTIVE, MENU_DIRECTIVE } from "@/ui/popover";
import { toaster } from "@/ui/toast";
import { EVENT_OUTSIDE_DIRECTIVE, HOVER_DIRECTIVE, TOOLTIP_DIRECTIVE } from "@/ui/tooltip";
import { COMMIT, ENV, GRPC_KEEPALIVE_INTERVAL_SECONDS, IS_DEV, SUPERVISOR_URL, VERSION } from "@/utils/globals";
import { log } from "@/utils/log";
import { registerViewComponents } from "@/views/registry";
import posthog from "posthog-js";
import { createApp } from "vue";
import Space from "./Space.vue";
import { watchCommands } from "@/ui/command";
import { HISTORY_COMMANDS } from "@/system/edit";
import { DEBUG_COMMANDS } from "@/system/debug";
import { RESOURCE_COMMANDS } from "@/language/resource/resource";

async function init() {
  const app = createApp(Space);

  // telemetry
  if (!IS_DEV) {
    posthog.init("phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma", {
      // public capture key
      api_host: "https://eu.posthog.com",
      enable_recording_console_log: true,
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
  if (IS_DEV) {
    app.config.performance = true;
  }
  app.config.errorHandler = (err, instance, info) => {
    log.error("error.internal", err, info);
    toaster.error({ title: "Internal client error", text: (err as any).message ?? info });
  };
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
  // (register commands that may not be imported directly)
  // eslint-disable-next-line @typescript-eslint/no-unused-expressions
  [HISTORY_COMMANDS, DEBUG_COMMANDS, RESOURCE_COMMANDS];

  app.mount("#app");
}

init();

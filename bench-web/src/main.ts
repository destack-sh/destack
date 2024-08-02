/* eslint-disable no-console */
import "./assets/index.css";

import { toaster } from "@/ui/toast";
import { setupTransactionManagement } from "@/language/transaction";
import { COMMIT, ENV, IS_DEV, SENTRY_DSN, SUPERVISOR_URL, VERSION } from "@/utils/globals";
import { keytrap } from "@/ui/keymap";
import { CONTEXT_MENU_DIRECTIVE, HOVER_MENU_DIRECTIVE, MENU_DIRECTIVE } from "@/ui/popover";
import { EVENT_OUTSIDE_DIRECTIVE, HOVER_DIRECTIVE, TOOLTIP_DIRECTIVE } from "@/ui/tooltip";
import { registerViewComponents } from "@/views/registry";
import * as Sentry from "@sentry/vue";
import posthog from "posthog-js";
import { createApp } from "vue";
import Space from "./Space.vue";
import { log } from "@/utils/log";

async function init() {
  const app = createApp(Space);

  // sentry / posthog instrumentation
  if (!IS_DEV) {
    posthog.init("phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma", {
      // public capture key
      api_host: "https://eu.posthog.com",
      enable_recording_console_log: true,
    });
    console.info("Setting up Sentry...", SENTRY_DSN != null);
    Sentry.init({
      app,
      dsn: import.meta.env.SENTRY_DSN,
      integrations: [
        Sentry.browserTracingIntegration({
          tracePropagationTargets: ["localhost", "127.0.0.1", "justbench.com", /^\//],
        }),
      ],
      tracesSampleRate: 1.0,
      logErrors: true,
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

  // set vue stuff
  if (IS_DEV) {
    app.config.performance = true;
  }
  app.config.errorHandler = (err, instance, info) => {
    log.error("error.internal", err, info);
    toaster.error({ title: "Internal client error", text: (err as any).message ?? info });
  };
  app.directive("tooltip", TOOLTIP_DIRECTIVE);
  app.directive("contextmenu", CONTEXT_MENU_DIRECTIVE);
  app.directive("menu", MENU_DIRECTIVE);
  app.directive("hovermenu", HOVER_MENU_DIRECTIVE);
  app.directive("hover", HOVER_DIRECTIVE);
  app.directive("outside", EVENT_OUTSIDE_DIRECTIVE);

  // start our own stuff
  await registerViewComponents();
  toaster.run();
  keytrap.track(document); // ensure it's always running
  setupTransactionManagement();

  app.mount("#app");
}

init();

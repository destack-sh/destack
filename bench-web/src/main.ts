import "./assets/index.css";

import { createApp } from "vue";
import Space from "./Space.vue";
import * as Sentry from "@sentry/vue";
import posthog from "posthog-js";
import { COMMIT, IS_DEBUG, SUPERVISOR_URL, VERSION } from "@/utils/globals";
import { createHead } from "@unhead/vue";
import { registerViewComponents } from "@/views";
import { toaster } from "@/system/toast";
import { keytrap } from "@/utils/keymap";

async function init() {
  const app = createApp(Space);
  app.use(createHead());

  // sentry / posthog instrumentation
  posthog.init("phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma", {
    // public capture key
    api_host: "https://eu.posthog.com",
    enable_recording_console_log: true,
  });
  if (!IS_DEBUG) {
    console.info("Setting up Sentry...", import.meta.env.VITE_APP_SENTRY_DSN != null);
    Sentry.init({
      app,
      dsn: import.meta.env.VITE_APP_SENTRY_DSN,
      integrations: [
        Sentry.browserTracingIntegration({
          tracePropagationTargets: ["localhost", "127.0.0.1", "api.justbench.com", /^\//],
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
  console.group(`%cBench OS`, "color:orangered");
  console.info(`%cVersion: ${VERSION} (${COMMIT})`, "color:orangered");
  console.info(`%cSupervisor: ${SUPERVISOR_URL}`, "color:orangered");
  console.info(`%cEnvironment: ${import.meta.env.MODE}`, "color:orangered");
  console.groupEnd();

  // prevent opening files that are dragged over the window
  window.addEventListener("dragover", (e) => e.preventDefault(), false);
  window.addEventListener("drop", (e) => e.preventDefault(), false);

  await registerViewComponents();

  if (IS_DEBUG) {
    app.config.performance = true;
  }
  app.config.errorHandler = (err, instance, info) => {
    console.error(err);
    toaster.error({ title: "Internal error", text: (err as any).message ?? info });
  };

  toaster.run();
  keytrap.track(document); // ensure it's always running
  // suppress save everywhere
  keytrap.bind(["ctrl+s", "mod+s"], () => {
    toaster.info({
      key: "space.suppressSave",
      icon: "fas fa-floppy-disk",
      title: "No need to save",
      text: "Bench synchronizes automatically.",
      debounce: true,
    });
    return true;
  });

  app.mount("#app");
}

init();

import { resetTransactionBuffers } from "@/language/core/transaction";
import { toaster } from "@/ui/toast";
import { IS_DEV } from "@/utils/globals";
import { log } from "@/utils/log";
import posthog from "posthog-js";

export function onUnhandledError(err: unknown) {
  if (typeof err === "string" && err.includes("ResizeObserver")) {
    return; // TODO :Robustness: don't just suppress ResizeObserver errors
  }
  log.error("error.internal", err);
  if (IS_DEV) {
    toaster.error({ title: "Internal client error", text: (err as any).message });
  }
  captureException(err);
}

export function captureException(err: unknown) {
  posthog.captureException(err);
  resetTransactionBuffers(); // NOTE: shouldn't we only do this if really needed?
}

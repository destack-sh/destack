import { log } from "@/utils/log";
import posthog from "posthog-js";

export function onUnhandledError(err: unknown) {
  if (typeof err === "string" && err.includes("ResizeObserver")) {
    return; // TODO :Robustness: don't just suppress ResizeObserver errors
  }
  log.error("error.internal", err);
  captureException(err);
}

export function captureException(err: unknown) {
  posthog.captureException(err);
}

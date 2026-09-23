import { describeService } from "@destack/service/inspect";
import { daemonService } from "../service/index.ts";

/** Describe daemon procedures without starting the host. */
export function inspect() {
    return describeService("daemon", daemonService);
}

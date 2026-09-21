import { implement } from "../server/handler.ts";
import type { Health } from "./health.ts";
import { health } from "./procedure.ts";

/** Implement health checks and streaming readiness updates. */
export function implementHealth(readiness: Health) {
    // read and subscribe to the same health state used by the host
    const implementation = implement(health);

    return implementation.router({
        check: implementation.check.handler(() => readiness.check()),
        watch: implementation.watch.handler(({ signal }) => readiness.watch(signal)),
    });
}

import { implement } from "./handler.ts";
import type { Health } from "../health/health.ts";
import { health } from "../health/procedure.ts";

/** Implement the health procedures. */
export function implementHealth(readiness: Health) {
    // implement the procedures
    const implementation = implement(health);

    return implementation.router({
        check: implementation.check.handler(() => readiness.check()),
        watch: implementation.watch.handler(({ signal }) => readiness.watch(signal)),
    });
}

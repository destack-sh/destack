import { eventIterator, defineProcedure } from "../service/service.ts";
import { HealthDescription } from "./health.ts";

/** Health procedures for one service instance. */
export const health = {
    check: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "GET", path: "/health" })
        .output(HealthDescription),
    watch: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "GET", path: "/health/watch" })
        .output(eventIterator(HealthDescription)),
};

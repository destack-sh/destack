import { eventIterator, defineProcedure } from "../service/service.ts";
import { HealthDescription } from "./health.ts";

/** The health procedures. */
export const health = {
    check: defineProcedure({ authentication: "public", permission: "anyone", audit: false })
        .route({ method: "GET", path: "/health" })
        .output(HealthDescription),
    watch: defineProcedure({ authentication: "public", permission: "anyone", audit: false })
        .route({ method: "GET", path: "/health/watch" })
        .output(eventIterator(HealthDescription)),
};

import { defineProcedure } from "./procedure.ts";
import { ServiceDescription } from "../inspect/service.ts";

/** Optional service reflection API, authorized by the hosting service. */
export const inspection = {
    get: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "GET", path: "/inspection" })
        .output(ServiceDescription),
};

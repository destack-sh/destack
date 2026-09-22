import { schema } from "@destack/schema";
import { eventIterator, defineProcedure } from "../service/service.ts";
import type { OperationDefinition } from "./operation.ts";

/** Define shared procedures for one typed family of long-running operations. */
export function defineOperationProcedures<Result, Progress>(
    definition: OperationDefinition<Result, Progress>,
    path: `/${string}` = "/operations",
) {
    // share typed operation values and request errors across all procedures
    const operation = definition.operation;
    const key = schema.object({ id: schema.uuid() });
    const request = defineProcedure({ authentication: "identity", permission: null, audit: false });

    return {
        get: request
            .route({ method: "GET", path: `${path}/{id}` })
            .input(key)
            .output(operation),
        list: request.route({ method: "GET", path }).output(schema.array(operation)),
        watch: request
            .route({ method: "GET", path: `${path}/{id}/watch` })
            .input(key)
            .output(eventIterator(operation)),
        cancel: request
            .route({ method: "POST", path: `${path}/{id}/cancel` })
            .input(key)
            .output(operation),
        delete: request
            .route({ method: "DELETE", path: `${path}/{id}` })
            .input(key)
            .output(schema.null()),
    };
}

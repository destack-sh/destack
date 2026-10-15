import { defineMethod, type Method } from "./method.ts";
import { Empty } from "./procedure.ts";

/** Remove an expired object at any revision, past the trash. */
export const expiry: Method = defineMethod<{ kind: "delete"; permission: "expire"; mutates: true }>(
    {
        kind: "delete",
        permission: "expire",
        isInternal: true,
        mutates: true,
        target: "object",
        result: "value",
        procedure: (_name, shapes) => ({
            route: { method: "DELETE", path: "/{id}" },
            input: shapes.target.extend(shapes.replay),
            output: Empty,
        }),
        handler: async (call) => {
            await call.remove();

            return {};
        },
    },
);

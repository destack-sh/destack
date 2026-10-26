import { defineMethod, type Method } from "./method.ts";
import { Empty } from "./procedure.ts";

/** Remove an expired object at any revision, past the trash. */
export const expiry: Method = defineMethod<{ kind: "delete"; permission: "expire"; mutates: true }>(
    {
        kind: "delete",
        permission: "expire",
        visibility: "package",
        mutates: true,
        target: "object",
        result: "value",
        procedure: (_name, schemas) => ({
            route: { method: "DELETE", path: "/{id}" },
            input: schemas.target.extend(schemas.replay),
            output: Empty,
        }),
        handler: async (context) => {
            await context.remove();

            return {};
        },
    },
);

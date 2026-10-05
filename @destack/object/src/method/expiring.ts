import type { Call } from "./call.ts";
import { defineMethod, type Method } from "./method.ts";
import { Empty } from "./procedure.ts";
import type { ObjectTable } from "../object/table.ts";

/** Remove an expired object at any revision, bypassing the trash. */
export const expiry: Method = defineMethod<{ kind: "delete"; permission: null; mutates: true }>({
    kind: "delete",
    permission: null,
    isSystem: true,
    mutates: true,
    target: true,
    result: "value",
    procedure: (_name, shapes) => ({
        route: { method: "DELETE", path: "/{id}" },
        input: shapes.target.extend(shapes.replay),
        output: Empty,
    }),
    handler: async (call: Call<ObjectTable>) => {
        await call.remove();

        return {};
    },
});

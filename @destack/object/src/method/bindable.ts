import { principal } from "@destack/access";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { defineMethod, type Method } from "./method.ts";
import type { Call } from "./call.ts";

import { CONSUMER } from "../trait/bindable.ts";
/** The input binding an installation: the installation and every object of the type bound to it in the scope. */
const BindInput = schema.object({
    /** The bound installation. */
    consumer: Subject,
    /** The objects of the type bound to it, every other one released. */
    ids: schema.array(schema.string().min(1)),
});

/** Bind exactly some objects of a type in a scope to an installation as its consumer, where their rows live. */
export const bind: Method<{
    kind: "bind";
    permission: null;
    mutates: true;
    system: true;
}> = defineMethod<{ kind: "bind"; permission: null; mutates: true; system: true }>({
    kind: "bind",
    permission: null,
    mutates: true,
    isSystem: true,
    target: false,
    result: "value",
    procedure: (_name, shapes) => ({
        route: { method: "POST", path: "/bind" },
        input: shapes.scope.extend(BindInput.shape),
        output: schema.object({}),
    }),
    handler: async (call: Call) => {
        // require an installation bound
        const { consumer, ids } = BindInput.parse(call.input);
        if (!principal.installation.is(consumer)) {
            throw new TypeError(`${consumer.id} is no installation binding ${call.object.name}`);
        }

        // keep exactly its consumer relationships on the type's objects in the scope
        const { packageId } = call.object.policy.definition;
        await call.requireAuthorization().keepRelationships(
            {
                scope: call.scope,
                objects: [{ packageId, type: call.object.name }],
                relation: CONSUMER,
                subject: consumer,
            },
            ids.map((id) => ({
                object: call.object.reference(call.scope, id),
                relation: CONSUMER,
                subject: consumer,
            })),
        );

        return {};
    },
});

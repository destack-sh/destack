import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { defineMethod, type Method } from "./method.ts";
import { MethodContext } from "./context.ts";

import { CONSUMER } from "../trait/bindable.ts";
import { Execution } from "./execution.ts";
/** The input binding an installation: the installation and all objects bound to it in scope. */
const BindInput = schema.object({
    /** The bound installation. */
    consumer: Subject,
    /** The objects of the type bound to it, every other one released. */
    ids: schema.array(schema.string().min(1)),
});

/** Bind exactly some objects of a type in a scope to an installation or machine as consumer. */
export const bind: Method<{
    kind: "bind";
    permission: "bind";
    mutates: true;
    internal: true;
    target: "scope";
}> = defineMethod<{
    kind: "bind";
    permission: "bind";
    mutates: true;
    internal: true;
    target: "scope";
}>({
    kind: "bind",
    permission: "bind",
    mutates: true,
    isInternal: true,
    target: "scope",
    result: "value",
    procedure: (_name, shapes) => ({
        route: { method: "POST", path: "/bind" },
        input: shapes.scope.extend(BindInput.shape),
        output: schema.object({}),
    }),
    handler: async (context: MethodContext, execution: Execution) => {
        // replace the consumer's relationships with exactly those on the type's objects in the scope
        const { consumer, ids } = BindInput.parse(context.input);
        const { packageId } = context.object.policy.definition;
        await Execution.store(execution, context).authorization.replaceRelationships(
            {
                scope: context.scope,
                objects: [{ packageId, type: context.object.name }],
                relation: CONSUMER,
                subject: consumer,
            },
            ids.map((id) => ({
                object: context.object.reference(context.scope, id),
                relation: CONSUMER,
                subject: consumer,
            })),
        );

        return {};
    },
});

/** The input binding or releasing one consumer of an object. */
export const ConsumerInput = schema.object({
    /** The consumer: an installation or a machine. */
    consumer: Subject,
});

/** A method binding or releasing one consumer of an object, called by a permission's holders. */
export type ConsumerMethod<Permission extends string> = Method<{
    kind: "custom";
    permission: Permission;
    input: typeof ConsumerInput;
    mutates: true;
}>;

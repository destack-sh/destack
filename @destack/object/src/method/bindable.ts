import { accessRelationship, principal } from "@destack/access";
import { and, eq } from "@destack/db";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { defineMethod, method, type Method } from "./method.ts";
import type { Call } from "./call.ts";

import { CONSUMER } from "../trait/bindable.ts";
/** The input binding an installation: the installation and every object of the type bound to it in the scope. */
const BindInput = schema.object({
    /** The bound installation. */
    consumer: Subject,
    /** The objects of the type bound to it, every other one released. */
    ids: schema.array(schema.string().min(1)),
});

/** Bind exactly some objects of a type in a scope to an installation, a cell or a placed workload as its consumer, where their rows live. */
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
        // require an installation, a cell or a placed workload bound
        const { consumer, ids } = BindInput.parse(call.input);
        if (
            !principal.installation.is(consumer) &&
            !principal.cell.is(consumer) &&
            !principal.workload.is(consumer)
        ) {
            throw new TypeError(
                `${consumer.id} is no installation, cell or workload binding ${call.object.name}`,
            );
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

/** The input binding or releasing one consumer of an object. */
const ConsumerInput = schema.object({
    /** The consumer: an installation, a cell or a placed workload. */
    consumer: Subject,
});

/** A method binding or releasing one consumer of an object, called by a permission's holders. */
export type ConsumerMethod<Permission extends string> = Method<{
    kind: "custom";
    permission: Permission;
    input: typeof ConsumerInput;
    mutates: true;
}>;

/** Declare the method binding one consumer to an object or releasing it, keeping the consumer's other objects of the type. */
export function consumerMethod<const Permission extends string>(
    permission: Permission,
    action: "bind" | "release",
): ConsumerMethod<Permission> {
    const inverse = action === "bind" ? "releaseConsumer" : "bindConsumer";
    const handled = method
        .mutation({ permission, input: ConsumerInput, inverse })
        .handle(async (call) => {
            // read the consumer's objects of the type in the scope
            const { consumer } = call.input;
            const { packageId } = call.object.policy.definition;
            const rows = await call.database
                .select({ objectId: accessRelationship.objectId })
                .from(accessRelationship)
                .where(
                    and(
                        eq(accessRelationship.scope, call.scope),
                        eq(accessRelationship.packageId, packageId),
                        eq(accessRelationship.type, call.object.name),
                        eq(accessRelationship.relation, CONSUMER),
                        eq(accessRelationship.subjectPackageId, consumer.packageId),
                        eq(accessRelationship.subjectType, consumer.type),
                        eq(accessRelationship.subjectScope, consumer.scope),
                        eq(accessRelationship.subjectId, consumer.id),
                    ),
                );

            // bind exactly them with the target added or removed, as the system binds
            const id = call.requireId();
            const kept = rows.map((row) => row.objectId).filter((each) => each !== id);
            const ids = action === "bind" ? [...kept, id] : kept;
            await call.change(call.object, "bind", { consumer, ids });

            return call.target;
        });

    return { ...handled, isPredicted: false };
}

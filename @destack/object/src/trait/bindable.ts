import { accessRelationship, principal } from "@destack/access";
import type { schema } from "@destack/schema";
import type { Procedure } from "../method/procedure.ts";
import { bind, ConsumerInput, type ConsumerMethod } from "../method/bindable.ts";
import { Call } from "../method/call.ts";
import { method } from "../method/record.ts";
import { type Gated, derivePermissions, type Trait } from "./trait.ts";

/** The relation of an object's consumers. */
export const CONSUMER = "consumer";

/** The permission binding consumers to the objects. */
const BIND = "bind";

/** The methods of bindable objects: binding them where their rows live, and one at a time. */
export type BindableMethodMap<Bindable> = [Bindable] extends [undefined]
    ? {}
    : [Bindable] extends [Gated<infer Permission>]
      ? {
            readonly bind: typeof bind;
            readonly bindConsumer: ConsumerMethod<Permission>;
            readonly releaseConsumer: ConsumerMethod<Permission>;
        }
      : { readonly bind: typeof bind };

/** The procedure binding derives, which stays out of the routes people call. */
export type BindableProcedures = {
    bind: Procedure<
        schema.Object<Readonly<Record<string, schema.Schema>>>,
        schema.Object<Readonly<Record<string, schema.Schema>>>
    >;
};

/** Objects that installations and machines bind to and consume, such as secrets. */
export const bindable: Trait<true | Gated> & {
    /** Declare the method binding or releasing one consumer's object, keeping its other objects. */
    readonly consumer: <const Permission extends string>(
        permission: Permission,
        action: "bind" | "release",
    ) => ConsumerMethod<Permission>;
} = {
    key: "bindable",
    storages: ["durable"],
    options: (definition) => definition.bindable,
    columns: () => ({}),
    constraints: () => [],
    policy: (_options, _object, permissions) => ({
        // derive the bind permission unless the type declares its own, as gated types do
        ...derivePermissions(permissions, [BIND]),
        relations: {
            [CONSUMER]: {
                subjects: [principal.installation, principal.machine],
                grantedBy: BIND,
            },
        },
    }),
    methods: (options) =>
        options === true
            ? { bind }
            : {
                  bind,
                  bindConsumer: bindable.consumer(options.by, "bind"),
                  releaseConsumer: bindable.consumer(options.by, "release"),
              },
    consumer<const Permission extends string>(
        permission: Permission,
        action: "bind" | "release",
    ): ConsumerMethod<Permission> {
        const inverse = action === "bind" ? "releaseConsumer" : "bindConsumer";
        return method
            .mutation({ permission, input: ConsumerInput, inverse, isPredicted: false })
            .handle(async (call) => {
                // read the consumer's objects of the type in the scope
                const { consumer } = call.input;
                const { packageId } = call.object.policy.definition;
                const rows = await call.read(accessRelationship).findMany({
                    where: {
                        scope: call.scope,
                        objectPackageId: packageId,
                        objectType: call.object.name,
                        relation: CONSUMER,
                        subjectPackageId: consumer.packageId,
                        subjectType: consumer.type,
                        subjectScope: consumer.scope,
                        subjectId: consumer.id,
                    },
                });

                // bind exactly them with the target added or removed, as the store's own work binds
                const id = call.requireId();
                const kept = rows.map((row) => row.objectId).filter((each) => each !== id);
                const ids = action === "bind" ? [...kept, id] : kept;
                await call.mutate(call.caller, [
                    Call.record(call.object, "bind", { consumer, ids }),
                ]);

                return call.target;
            });
    },
};

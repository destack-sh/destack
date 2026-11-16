import { accessRelationship, principal, Relationship } from "@destack/access";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { MethodContext } from "../method/context.ts";
import { Execution } from "../method/execution.ts";
import { defineMethod, type Method } from "../method/method.ts";
import type { Procedure } from "../method/procedure.ts";
import { method } from "../method/builder.ts";
import {
    type TraitPermission,
    derivePermissions,
    type TraitBehavior,
    callableTrait,
} from "./trait.ts";
import { t } from "@destack/locale";

/** The relation of an object's consumers. */
export const CONSUMER = "consumer";

/** The permission binding consumers to the objects. */
const BIND = "bind";

/** The input binding an installation: the installation and all objects bound to it in scope. */
const BindInput = schema.object({
    /** The bound installation. */
    consumer: Subject,
    /** The objects of the type bound to it, every other one released. */
    ids: schema.array(schema.string().min(1)),
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

/** The methods of bindable objects: binding them where their rows live, and one at a time. */
export type BindableMethodMap<Options> = [Options] extends [undefined]
    ? {}
    : [Options] extends [TraitPermission<infer Permission>]
      ? {
            readonly bind: typeof Bindable.bind;
            readonly bindConsumer: ConsumerMethod<Permission>;
            readonly releaseConsumer: ConsumerMethod<Permission>;
        }
      : { readonly bind: typeof Bindable.bind };

/** The procedure binding derives, which stays out of the routes people call. */
export type BindableProcedures = {
    bind: Procedure<
        schema.Object<Readonly<Record<string, schema.Schema>>>,
        schema.Object<Readonly<Record<string, schema.Schema>>>
    >;
};

/** Objects that installations and machines bind to and consume, such as secrets. */
const bindableTrait: TraitBehavior<true | TraitPermission> = {
    key: "bindable",
    storages: ["durable"],
    options: (definition) => definition.bindable,
    columns: () => ({}),
    constraints: () => [],
    policy: (_options, _object, permissions) => ({
        // derive the bind permission unless the type declares it, as gated types do
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
            ? { bind: Bindable.bind }
            : {
                  bind: Bindable.bind,
                  bindConsumer: Bindable.consumer(options.by, "bind"),
                  releaseConsumer: Bindable.consumer(options.by, "release"),
              },
};

/** Let installations and machines bind to and consume objects, such as secrets. */
export const bindable = callableTrait("bindable", bindableTrait);

/** The binding of objects to their consumers: all of a consumer's at once, or one at a time. */
export const Bindable = {
    /** Bind exactly some objects of a type in a scope to an installation or machine as consumer. */
    bind: defineMethod<{
        kind: "bind";
        permission: "bind";
        mutates: true;
        visibility: "installation";
        target: "scope";
    }>({
        title: t`Bind`,
        kind: "bind",
        permission: "bind",
        mutates: true,
        visibility: "installation",
        target: "scope",
        result: "value",
        procedure: (_name, schemas) => ({
            route: { method: "POST", path: "/bind" },
            input: schemas.scope.extend(BindInput.shape),
            output: schema.object({}),
        }),
        handler: async (context: MethodContext, execution: Execution) => {
            // replace the consumer's relationships with exactly those on the type's objects in the scope
            const { consumer, ids } = BindInput.parse(context.input);
            const { packageId } = context.object.policy.definition;
            await Relationship.replace(
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
                Execution.store(execution, context).authorization,
            );

            return {};
        },
    }),

    /** Declare the method binding or releasing one consumer's object, leaving its other objects. */
    consumer<const Permission extends string>(
        permission: Permission,
        action: "bind" | "release",
    ): ConsumerMethod<Permission> {
        const inverse = action === "bind" ? "releaseConsumer" : "bindConsumer";

        return method
            .mutation({
                title: action === "bind" ? t`Bind consumer` : t`Release consumer`,
                permission,
                input: ConsumerInput,
                inverse,
                isPredicted: false,
            })
            .handle(async (context) => {
                // read the consumer's objects of the type in the scope
                const { consumer } = context.input;
                const { packageId } = context.object.policy.definition;
                const rows = await context.read(accessRelationship).findMany({
                    where: {
                        scope: context.scope,
                        objectPackageId: packageId,
                        objectType: context.object.name,
                        relation: CONSUMER,
                        subjectPackageId: consumer.packageId,
                        subjectType: consumer.type,
                        subjectScope: consumer.scope,
                        subjectId: consumer.id,
                    },
                });

                // bind exactly them with the target added or removed, as the store's work binds
                const id = context.id;
                const others = rows.map((row) => row.objectId).filter((each) => each !== id);
                const ids = action === "bind" ? [...others, id] : others;
                await context.mutate(context.caller, [
                    context.object.call("bind", { consumer, ids }),
                ]);

                return context.target;
            });
    },
};

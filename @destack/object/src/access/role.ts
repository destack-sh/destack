import * as access from "@destack/access";
import { accessRole, Role, RoleRequest } from "@destack/access";
import { Empty } from "../method/procedure.ts";
import { ObjectReference, Subject } from "@destack/sync";
import { present, schema } from "@destack/schema";
import { defineMethod } from "../method/method.ts";
import { type MethodBuilder } from "../method/record.ts";
import { defineObject } from "../object/object.ts";
import { Execution, type StoreExecution } from "../method/execution.ts";
import { ObjectPolicy } from "../object/access.ts";
import { type Select } from "@destack/db";

/** A role upserted by name under a manager, granting exactly some permissions to one subject. */
const RoleBinding = RoleRequest.extend({
    /** The one subject the role is bound to. */
    subject: Subject,
    /** The object in the scope the role is bound on, the scope itself when absent. */
    object: ObjectReference.exactOptional(),
    /** The stack declaration managing the role and its binding. */
    manager: access.Manager.schema,
});

/** A role code owns in a scope, bound to one subject. */
const InherentRole = RoleRequest.extend({
    /** The one subject the role is bound to. */
    subject: Subject,
});

/** The roles of a scope, over the access role table. */
export const role = defineObject(
    {
        name: "role",
        plural: "roles",
        mapping: ObjectPolicy.readMapping(accessRole),
        represents: access.role,
        permissions: access.role.definition.permissions,
    },
    (method: MethodBuilder<typeof accessRole>) => ({
        get: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof Empty;
            output: typeof access.Role.schema;
            mutates: false;
            target: "object";
            table: typeof accessRole;
        }>({
            kind: "custom",
            permission: "read",
            input: Empty,
            output: access.Role.schema,
            mutates: false,
            target: "object",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "GET", path: "/{id}" },
                input: schemas.target,
                output: access.Role.schema,
            }),
            handler: async (context, execution) => {
                // read the role with the permissions it grants
                const { transaction } = Execution.store(execution, context);
                const { id, scope, name, description, revision } = context.target;
                const permissions = await Role.permissions(transaction, id);

                return { id, scope, name, description, permissions, revision };
            },
        }),
        create: Execution.handle(
            method.create("create", {
                fields: ["name", "description"],
                input: RoleRequest.pick({ permissions: true }),
                isPredicted: false,
            }),
            async (call, execution) => {
                // create the role through the access role API
                const { name, description, permissions } = call.input;
                const store = Execution.store(execution, call);
                const created = await Role.create(
                    await ObjectPolicy.scopeObject(call, execution),
                    { name, description, permissions },
                    store.authorization,
                );

                return readRole(store, created.id);
            },
        ),
        update: Execution.handle(
            method.update("update", {
                fields: ["name", "description"],
                input: schema.object({
                    permissions: RoleRequest.shape.permissions.exactOptional(),
                }),
                isPredicted: false,
            }),
            async (call, execution) => {
                // change the given values at the loaded revision
                const { name, description, permissions } = call.input;
                const store = Execution.store(execution, call);
                await Role.update(
                    await ObjectPolicy.scopeObject(call, execution),
                    call.target.id,
                    {
                        ...(name === undefined ? {} : { name }),
                        ...(description === undefined ? {} : { description }),
                        ...(permissions === undefined ? {} : { permissions }),
                        revision: call.target.revision,
                    },
                    store.authorization,
                );

                return readRole(store, call.target.id);
            },
        ),
        upsertBinding: defineMethod<{
            kind: "custom";
            permission: "update";
            input: typeof RoleBinding;
            output: typeof Empty;
            mutates: true;
            target: "scope";
            table: typeof accessRole;
        }>({
            kind: "custom",
            permission: "update",
            input: RoleBinding,
            output: Empty,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "/upsert-binding" },
                input: schemas.scope.extend({ ...schemas.replay, ...RoleBinding.shape }),
                output: Empty,
            }),
            handler: async (context, execution) => {
                // bind the role under the manager with exactly its permissions, or drop both for none
                const { manager, ...binding } = RoleBinding.parse(context.input);
                await Role.upsertBinding(
                    await ObjectPolicy.scopeObject(context, execution),
                    binding,
                    Execution.store(execution, context).authorization.applying(manager),
                );

                return {};
            },
        }),
        applyInherent: defineMethod<{
            kind: "custom";
            permission: "inherit";
            input: typeof InherentRole;
            output: typeof Empty;
            mutates: true;
            target: "scope";
            table: typeof accessRole;
        }>({
            kind: "custom",
            permission: "inherit",
            input: InherentRole,
            output: Empty,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            visibility: "installation",
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "/apply-inherent" },
                input: schemas.scope.extend({ ...schemas.replay, ...InherentRole.shape }),
                output: Empty,
            }),
            handler: async (context, execution) => {
                // define the code-owned role with exactly its permissions, bound to its subject
                const { subject, ...request } = InherentRole.parse(context.input);
                await Role.applyInherent(
                    await ObjectPolicy.scopeObject(context, execution),
                    request,
                    subject,
                    Execution.store(execution, context).authorization,
                );

                return {};
            },
        }),
        delete: Execution.handle(
            method.delete("delete", { isPredicted: false }),
            async (call, execution) => {
                // delete the unbound role at the loaded revision
                await Role.delete(
                    await ObjectPolicy.scopeObject(call, execution),
                    call.target.id,
                    call.target.revision,
                    Execution.store(execution, call).authorization,
                );

                return {};
            },
        ),
    }),
);

/** Read one role row as a method's result. */
async function readRole(
    execution: StoreExecution,
    id: Select<typeof accessRole>["id"],
): Promise<Select<typeof accessRole>> {
    return present(
        await execution.transaction.read(accessRole).findFirst({ where: { id } }),
        `role ${id}`,
    );
}

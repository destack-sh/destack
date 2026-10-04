import * as access from "@destack/access";
import { Scope, type ObjectReference } from "@destack/sync";
import { Snapshot, TABLE, eq, type Select, type Table } from "@destack/db";
import {
    ACCESS_MAPPINGS,
    accessProposal,
    accessRelationship,
    accessRole,
    PermissionReference,
    type TableMapping,
} from "@destack/access";
import { present, schema } from "@destack/schema";
import { type Call } from "../method/call.ts";
import type { Method, MethodBuilder } from "../method/method.ts";
import { INTRINSIC } from "./intrinsic.ts";
import type { ObjectScope } from "./object.ts";

/** The permissions a role grants. */
const RolePermissions = schema.object({
    /** The permissions the role grants. */
    permissions: schema.array(PermissionReference),
});

/** Define a scope type's roles as objects. */
export function role<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "role",
        plural: "roles",
        [INTRINSIC]: { table: accessRole, mapping: mappingOf(accessRole) },
        scope,
        represents: access.role,
        permissions: access.role.definition.permissions,
        methods: (method: MethodBuilder<typeof accessRole>) => ({
            get: method.get("read"),
            list: method.list("read"),
            create: unpredicted(
                method.create("create", {
                    fields: ["name", "description"],
                    input: RolePermissions,
                }),
            ).handle(async (call) => {
                // create the role through the access role API
                const { name, description, permissions } = call.input;
                const created = await call
                    .requireAuthorization()
                    .createRole(await scopeObject(call), { name, description, permissions });

                return readRow(call, created.id);
            }),
            update: unpredicted(
                method.update("update", {
                    fields: ["name", "description"],
                    input: schema.object({
                        permissions: schema.array(PermissionReference).exactOptional(),
                    }),
                }),
            ).handle(async (call) => {
                // change the given values at the loaded revision
                const { name, description, permissions } = call.input;
                await call
                    .requireAuthorization()
                    .updateRole(await scopeObject(call), call.target.id, {
                        ...(name === undefined ? {} : { name }),
                        ...(description === undefined ? {} : { description }),
                        ...(permissions === undefined ? {} : { permissions }),
                        revision: call.target.revision,
                    });

                return readRow(call, call.target.id);
            }),
            delete: unpredicted(method.delete("delete")).handle(async (call) => {
                // delete the unbound role at the loaded revision
                await call
                    .requireAuthorization()
                    .deleteRole(await scopeObject(call), call.target.id, call.target.revision);

                return {};
            }),
        }),
    } as const;
}

/** Define a scope type's relationships as objects, changed only through sharing methods. */
export function relationship<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "relationship",
        plural: "relationships",
        [INTRINSIC]: { table: accessRelationship, mapping: mappingOf(accessRelationship) },
        scope,
        represents: access.relationship,
        permissions: access.relationship.definition.permissions,
        methods: (method: MethodBuilder<typeof accessRelationship>) => ({
            get: method.get("read"),
            list: method.list("read"),
        }),
    } as const;
}

/** Define a scope type's proposals as objects, answered through sharing methods. */
export function proposal<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "proposal",
        plural: "proposals",
        [INTRINSIC]: { table: accessProposal, mapping: mappingOf(accessProposal) },
        scope,
        represents: access.proposal,
        permissions: access.proposal.definition.permissions,
        methods: (method: MethodBuilder<typeof accessProposal>) => ({
            get: method.get("read"),
            list: method.list("read"),
        }),
    } as const;
}

/** Keep a method on the server. */
function unpredicted<Declared extends Method>(declared: Declared): Declared {
    return { ...declared, isPredicted: false };
}

/** Name the scope object of the call's scope. */
function scopeObject(call: Call): Promise<ObjectReference> {
    return Scope.object(Snapshot.live(call.database), call.scope);
}

/** Read one role row as the method's result. */
async function readRow(
    call: Call,
    id: Select<typeof accessRole>["id"],
): Promise<Select<typeof accessRole>> {
    const [row] = await call.database.select().from(accessRole).where(eq(accessRole.id, id));

    return present(row, `role ${id}`);
}

/** Read how access maps one of its own tables. */
function mappingOf(table: Table): TableMapping {
    return present(
        ACCESS_MAPPINGS.find((mapping) => mapping.table === table),
        `the access mapping of ${table[TABLE].name}`,
    );
}

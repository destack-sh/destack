import * as access from "@destack/access";
import { Snapshot } from "@destack/db/log";
import {
    ACCESS_MAPPINGS,
    accessProposal,
    accessRelationship,
    accessRole,
    PermissionReference,
    type ObjectReference,
    type TableMapping,
    Scope,
} from "@destack/access";
import { eq, type Table } from "@destack/db";
import { schema } from "@destack/schema";
import { type Call } from "../method/call.ts";
import { method, type Method } from "../method/method.ts";
import { INTRINSIC } from "./intrinsic.ts";
import type { ObjectScope } from "./object.ts";

/** The permissions a role grants. */
const RolePermissions = schema.object({
    /** The permissions the role grants. */
    permissions: schema.array(PermissionReference),
});

/** A role's input. */
type RoleInput = { readonly name: string; readonly description: string } & schema.Infer<
    typeof RolePermissions
>;

/** Define a scope type's roles as objects. */
export function role<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "role",
        plural: "roles",
        [INTRINSIC]: { table: accessRole, mapping: mappingOf(accessRole) },
        scope,
        represents: access.role,
        permissions: access.role.definition.permissions,
        methods: {
            get: method.get("read"),
            list: method.list("read"),
            create: unpredicted(
                method.create("create", {
                    fields: ["name", "description"],
                    input: RolePermissions,
                }),
            ).handle(createRole),
            update: unpredicted(
                method.update("update", {
                    fields: ["name", "description"],
                    input: RolePermissions.partial(),
                }),
            ).handle(updateRole),
            delete: unpredicted(method.delete("delete")).handle(deleteRole),
        },
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
        methods: { get: method.get("read"), list: method.list("read") },
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
        methods: { get: method.get("read"), list: method.list("read") },
    } as const;
}

/** Keep a method on the server. */
function unpredicted<Declared extends Method>(declared: Declared): Declared {
    return { ...declared, isPredicted: false };
}

/** Create a role through the access role API. */
async function createRole(call: Call): Promise<unknown> {
    const { name, description, permissions } = call.input as RoleInput;
    const created = await call.served().createRole(await scopeObject(call), {
        name,
        description,
        permissions,
    });

    return readRow(call, created.id);
}

/** Update a role at the call's revision. */
async function updateRole(call: Call): Promise<unknown> {
    // change the named values at the loaded revision
    const target = call.target as { readonly id: string; readonly revision: number };
    const { name, description, permissions } = call.input as Partial<RoleInput>;
    await call.served().updateRole(await scopeObject(call), target.id, {
        ...(name === undefined ? {} : { name }),
        ...(description === undefined ? {} : { description }),
        ...(permissions === undefined ? {} : { permissions }),
        revision: target.revision,
    });

    return readRow(call, target.id);
}

/** Delete an unbound role at the call's revision. */
async function deleteRole(call: Call): Promise<unknown> {
    const target = call.target as { readonly id: string; readonly revision: number };
    await call.served().deleteRole(await scopeObject(call), target.id, target.revision);

    return {};
}

/** Name the scope object of the call's scope. */
function scopeObject(call: Call): Promise<ObjectReference> {
    return Scope.object(Snapshot.live(call.database), call.scope);
}

/** Read one role row as the method's result. */
async function readRow(call: Call, id: string): Promise<Record<string, unknown>> {
    const [row] = await call.database
        .select()
        .from(accessRole)
        .where(eq(accessRole.id, id as never));

    return row as Record<string, unknown>;
}

/** Read how access maps one of its own tables. */
function mappingOf(table: Table): TableMapping {
    return ACCESS_MAPPINGS.find((mapping) => mapping.table === table)!;
}

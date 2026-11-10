import { schema } from "@destack/schema";
import type { GrantPermission } from "../grant/permission.ts";
import { Permission } from "./policy.ts";

/** A scope relative to a delegator's context: its space, account, universe, or a user's home. */
export const PermissionScope = schema.enum(["space", "account", "universe", "home"]);
/** A scope relative to a delegator's context. */
export type PermissionScope = schema.Infer<typeof PermissionScope>;

/** The schema of a permission set: permissions by the scope they apply in. */
const permissionSetSchema = schema
    .object({
        /** The permissions in the partition the code runs in: its space, machine or device. */
        space: schema.array(Permission).exactOptional(),
        /** The permissions in the account of that space. */
        account: schema.array(Permission).exactOptional(),
        /** The permissions on the universe's objects, such as users and clients. */
        universe: schema.array(Permission).exactOptional(),
        /** The permissions in a user's home: their personal scope and their home space. */
        home: schema.array(Permission).exactOptional(),
    })
    .strict();

/** Permissions by the scope they apply in, relative to the delegator's context. */
export const PermissionSet = Object.assign(permissionSetSchema, {
    /** Grant a set's permissions in the context's scopes, each with its scope in the set. */
    grant(set: PermissionSet, context: GrantContext): PermissionEntry[] {
        return PermissionScope.options.flatMap((within): PermissionEntry[] => {
            // grant the universe's permissions in every scope of the delegator
            if (within === "universe") {
                return (set.universe ?? []).map((permission) => ({
                    within,
                    permission: { ...permission },
                }));
            }

            // grant the scope's permissions in each matching scope of the context
            const named = context[within];
            const each = named === undefined ? [] : typeof named === "string" ? [named] : named;

            return each.flatMap((scope) =>
                (set[within] ?? []).map((permission) => ({
                    within,
                    permission: { ...permission, scope },
                })),
            );
        });
    },
});
/** Permissions by the scope they apply in. */
export type PermissionSet = schema.Infer<typeof permissionSetSchema>;

/** A permission granted from a set, with the set's scope it came from. */
export interface PermissionEntry {
    /** The set's scope: the space, the person's home, the account, or the universe. */
    readonly within: PermissionScope;
    /** The permission, granted in a scope of the context. */
    readonly permission: GrantPermission;
}

/** The scopes of a delegator's context a permission set is granted in. */
export interface GrantContext {
    /** The space. */
    readonly space: string;
    /** The person's home scopes: their user and their home space. */
    readonly home: readonly string[];
    /** The space's account, absent when unknown. */
    readonly account: string | undefined;
}

/** What a space grants an installation at install: in the space, account and universe. */
const SpaceSet = permissionSetSchema.pick({ space: true, account: true, universe: true });

/** What a package or its capability may be granted by each delegator, at install or on use. */
export const GrantRequest = schema
    .object({
        /** What the installation's space grants it at install. */
        space: SpaceSet.exactOptional(),
        /** What each user grants it on use, as when they open its view or start its agent. */
        user: permissionSetSchema.exactOptional(),
    })
    .strict();
/** What a package may be granted, by delegator. */
export type GrantRequest = schema.Infer<typeof GrantRequest>;

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
        /** The permissions in the space the code runs in. */
        space: schema.array(Permission).exactOptional(),
        /** The permissions in the account of that space. */
        account: schema.array(Permission).exactOptional(),
        /** The permissions on the universe's objects, such as users and clients. */
        universe: schema.array(Permission).exactOptional(),
        /** The permissions in a user's home: their own scope and their home space. */
        home: schema.array(Permission).exactOptional(),
    })
    .strict();

/** Permissions by the scope they apply in, relative to the delegator's context. */
export const PermissionSet = Object.assign(permissionSetSchema, {
    /** Grant a set's permissions in the context's scopes, the universe's in every scope. */
    grant(
        set: PermissionSet,
        scopes: {
            readonly space: string;
            readonly home: readonly string[];
            readonly account: string | undefined;
        },
    ): GrantPermission[] {
        return PermissionScope.options.flatMap((name) => {
            // grant the universe's permissions in every scope the delegator reaches
            if (name === "universe") {
                return (set.universe ?? []).map((permission) => ({ ...permission }));
            }

            // grant the scope's permissions in each matching scope of the context
            const named = scopes[name];
            const each = named === undefined ? [] : typeof named === "string" ? [named] : named;

            return each.flatMap((scope) =>
                (set[name] ?? []).map((permission) => ({ ...permission, scope })),
            );
        });
    },
});
/** Permissions by the scope they apply in. */
export type PermissionSet = schema.Infer<typeof PermissionSet>;

/** What an installation's space lends it at install: in the space, account and universe. */
const InstallationSet = permissionSetSchema.pick({ space: true, account: true, universe: true });

/** What a user lends an installation on use: in the space, the account and their home. */
const UserSet = permissionSetSchema.pick({ space: true, account: true, home: true });

/** What a package or its capability requests by delegator, consented at install or on use. */
export const PermissionRequest = schema
    .object({
        /** What the installation's space lends it at install. */
        installation: InstallationSet.exactOptional(),
        /** What each user lends it on use, as when they open its view or start its agent. */
        user: UserSet.exactOptional(),
    })
    .strict();
/** What a package's installations request, by delegator. */
export type PermissionRequest = schema.Infer<typeof PermissionRequest>;

import { defineSchema, schema } from "@destack/schema";
import type { GrantPermission } from "../grant/permission.ts";
import { Permission } from "./policy.ts";

/** A scope relative to a view's or package's context: space, home, account, or universe. */
export const PermissionScope = defineSchema(schema.enum(["space", "home", "account", "universe"]));
/** A scope relative to a view's or package's context. */
export type PermissionScope = schema.Infer<typeof PermissionScope>;

/** A view's or package's requested permissions by scope, consented at install, granted exactly. */
export const PermissionRequest = Object.assign(
    defineSchema(
        schema
            .object({
                /** The permissions in the space it runs in. */
                space: schema.array(Permission).exactOptional(),
                /** The permissions in the person's home: their own scope and their home space. */
                home: schema.array(Permission).exactOptional(),
                /** The permissions in the account of its space. */
                account: schema.array(Permission).exactOptional(),
                /** The permissions on the universe's objects, such as users and accounts. */
                universe: schema.array(Permission).exactOptional(),
            })
            .strict(),
    ),
    {
        /** Grant requested permissions in their scopes, the universe's in every scope. */
        grant(
            request: PermissionRequest,
            scopes: {
                readonly space: string;
                readonly home: readonly string[];
                readonly account: string | undefined;
            },
        ): GrantPermission[] {
            return PermissionScope.options.flatMap((name) => {
                // grant the universe's permissions in every scope the delegator reaches
                if (name === "universe") {
                    return (request.universe ?? []).map((permission) => ({ ...permission }));
                }

                // grant the scope's permissions in each matching scope of the context
                const named = scopes[name];
                const each = named === undefined ? [] : typeof named === "string" ? [named] : named;

                return each.flatMap((scope) =>
                    (request[name] ?? []).map((permission) => ({ ...permission, scope })),
                );
            });
        },
    },
);
/** The permissions a view or package requests by the scope they apply in. */
export type PermissionRequest = schema.Infer<typeof PermissionRequest>;

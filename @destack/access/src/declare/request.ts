import { defineSchema, schema } from "@destack/schema";
import { PermissionReference } from "./policy.ts";

/** A scope relative to a view's or workload's context: space, person's home, or account. */
export const PermissionScope = defineSchema(schema.enum(["space", "home", "account"]));
/** A scope relative to a view's or workload's context. */
export type PermissionScope = schema.Infer<typeof PermissionScope>;

/** A view's or workload's requested permissions by scope, consented at install, granted exactly. */
export const PermissionRequest = Object.assign(
    defineSchema(
        schema
            .object({
                /** The permissions in the space it runs in. */
                space: schema.array(PermissionReference).exactOptional(),
                /** The permissions in the person's home: their own scope and their home space. */
                home: schema.array(PermissionReference).exactOptional(),
                /** The permissions in the account of its space. */
                account: schema.array(PermissionReference).exactOptional(),
            })
            .strict(),
    ),
    {
        /** Grant requested permissions in their scopes, dropping scopes the context lacks. */
        grant(
            request: PermissionRequest,
            scopes: {
                readonly space: string;
                readonly home: readonly string[];
                readonly account: string | undefined;
            },
        ): (PermissionReference & { readonly scope: string })[] {
            return PermissionScope.options.flatMap((name) => {
                // grant the scope's permissions in each scope the context names for it
                const named = scopes[name];
                const each = named === undefined ? [] : typeof named === "string" ? [named] : named;

                return each.flatMap((scope) =>
                    (request[name] ?? []).map((permission) => ({ ...permission, scope })),
                );
            });
        },
    },
);
/** The permissions a view or workload requests by the scope they apply in. */
export type PermissionRequest = schema.Infer<typeof PermissionRequest>;

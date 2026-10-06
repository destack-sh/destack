import { defineSchema, schema } from "@destack/schema";
import { PermissionReference } from "./policy.ts";

/** A scope relative to a view's or workload's context: its space, the person's home, or the space's account. */
export const PermissionScope = defineSchema(schema.enum(["space", "home", "account"]));
/** A scope relative to a view's or workload's context. */
export type PermissionScope = schema.Infer<typeof PermissionScope>;

/** The permissions a view or workload requests by the scope they apply in, consented to at install and granted exactly. */
export const PermissionRequest = Object.assign(
    defineSchema(
        schema
            .object({
                /** The permissions in the space it runs in. */
                space: schema.array(PermissionReference).exactOptional(),
                /** The permissions in the person's home. */
                home: schema.array(PermissionReference).exactOptional(),
                /** The permissions in the account of its space. */
                account: schema.array(PermissionReference).exactOptional(),
            })
            .strict(),
    ),
    {
        /** Grant requested permissions in the scopes they name, dropping those of a scope the context lacks. */
        grant(
            request: PermissionRequest,
            scopes: {
                readonly space: string;
                readonly home: string | undefined;
                readonly account: string | undefined;
            },
        ): (PermissionReference & { readonly scope: string })[] {
            return PermissionScope.options.flatMap((name) => {
                // grant the scope's permissions where the context names the scope
                const scope = scopes[name];

                return scope === undefined
                    ? []
                    : (request[name] ?? []).map((permission) => ({ ...permission, scope }));
            });
        },
    },
);
/** The permissions a view or workload requests by the scope they apply in. */
export type PermissionRequest = schema.Infer<typeof PermissionRequest>;

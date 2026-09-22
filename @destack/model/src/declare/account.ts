import { PermissionReference } from "@destack/access/declare";
import { Tags } from "@destack/db";
import { ResourceName } from "@destack/resource";
import { defineSchema, identifier, schema } from "@destack/schema";
import { ModelError } from "../error/index.ts";

/** An account-defined environment; its declaration key is its stable name. */
export const EnvironmentDefinition = defineSchema(
    schema.object({
        /** Additional account-defined metadata. */
        tags: Tags.optional(),
    }),
);

/** A permission restricted to the account or one object within it. */
export const AccountPermission = defineSchema(
    PermissionReference.extend({
        /** The selected object; absence selects all objects in the binding's scope. */
        objectId: schema.string().min(1).optional(),
    }),
);

/** An account role declared in source. */
export const AccountRole = defineSchema(
    schema.object({
        /** The role's purpose. */
        description: schema.string().min(1),
        /** Permissions evaluated within the role binding's scope. */
        permissions: schema.array(AccountPermission),
    }),
);

/** A grant to an existing account member, group, or service account. */
export const AccountRoleBinding = defineSchema(
    schema.object({
        /** A declared role name or an existing role in this account. */
        role: schema.union([ResourceName, schema.object({ id: identifier("role") })]),
        /** The existing subject in this account. */
        subject: schema.union([
            schema.object({ membership: identifier("account-membership") }),
            schema.object({ group: identifier("group") }),
            schema.object({ service: identifier("service-account") }),
        ]),
        /** An optional space restriction within the account. */
        spaceId: identifier("space").optional(),
        /** Optional expiry in UTC epoch milliseconds. */
        expiresAt: schema.number().int().nonnegative().optional(),
    }),
);

/** Account records declared by a repository export. */
export const AccountDefinition = defineSchema(
    schema.object({
        /** Account-local environment names. */
        environments: schema.record(ResourceName, EnvironmentDefinition).optional(),
        /** Account roles available to bindings. */
        roles: schema.record(ResourceName, AccountRole).optional(),
        /** Grants applied using the caller's existing authority. */
        bindings: schema.record(ResourceName, AccountRoleBinding).optional(),
    }),
);

/** Account records declared by a repository export. */
export type AccountDefinition = schema.Infer<typeof AccountDefinition>;

/** Declare account records without creating or changing an account. */
export function defineAccount(
    definition: schema.Input<typeof AccountDefinition>,
): AccountDefinition {
    const account = AccountDefinition.parse(definition);

    // require named roles to exist in the same declaration
    for (const binding of Object.values(account.bindings ?? {})) {
        if (typeof binding.role === "string" && !Object.hasOwn(account.roles ?? {}, binding.role)) {
            throw new ModelError("INVALID_DEFINITION", `unknown role: ${binding.role}`);
        }
    }

    return account;
}

import { TagMap } from "@destack/object";
import { PermissionReference } from "@destack/access/declare";
import {
    DeclarationName,
    declaringModule,
    type ModuleMetadata,
    type Package,
} from "@destack/package";
import { defineSchema, identifier, schema } from "@destack/schema";
import { AccountError } from "../error/error.ts";

/** An account-defined environment. */
export const EnvironmentDefinition = defineSchema(
    schema.object({
        /** Additional account-defined metadata. */
        tags: TagMap.optional(),
    }),
);

/** An account role declared in source. */
export const AccountRole = defineSchema(
    schema.object({
        /** The role's purpose. */
        description: schema.string().min(1),
        /** Permissions granted wherever the role is bound. */
        permissions: schema.array(PermissionReference),
    }),
);

/** A role granted to a user, a group or a service account of the account. */
export const AccountRoleBinding = defineSchema(
    schema.object({
        /** A declared role name or an existing role in this account. */
        role: schema.union([DeclarationName, schema.object({ id: identifier("role") })]),
        /** The user, group or service account receiving the role. */
        subject: schema.union([
            schema.object({ user: identifier("user") }),
            schema.object({ group: identifier("group") }),
            schema.object({ service: identifier("service-account") }),
        ]),
        /** The space the role is bound on. */
        spaceId: identifier("space").optional(),
        /** Optional expiry in UTC epoch milliseconds. */
        expiresAt: schema.number().int().nonnegative().optional(),
    }),
);

/** Account records declared by a repository export. */
export const AccountDefinition = defineSchema(
    schema.object({
        /** Account-local environment names. */
        environments: schema.record(DeclarationName, EnvironmentDefinition).optional(),
        /** Account roles available to bindings. */
        roles: schema.record(DeclarationName, AccountRole).optional(),
        /** Grants applied using the caller's existing authority. */
        bindings: schema.record(DeclarationName, AccountRoleBinding).optional(),
    }),
);

/** Account records declared by a repository export. */
export type AccountDefinition = schema.Infer<typeof AccountDefinition>;

/** Account records with the package declaring them. */
export interface AccountDeclaration {
    /** The declaring package, supplied by the module transform. */
    readonly package: Package;
    /** The declared records. */
    readonly definition: AccountDefinition;
}

/** Declare account records without creating or changing an account. */
export function defineAccount(
    definition: schema.Input<typeof AccountDefinition>,
    module?: ModuleMetadata,
): AccountDeclaration {
    // stamp the declaring package and validate the records
    const owner = declaringModule(module, "defineAccount").package;
    const account = AccountDefinition.parse(definition);

    // require named roles to exist in the same declaration
    for (const binding of Object.values(account.bindings ?? {})) {
        if (typeof binding.role === "string" && !Object.hasOwn(account.roles ?? {}, binding.role)) {
            throw new AccountError("INVALID_DEFINITION", `unknown role: ${binding.role}`);
        }
    }

    return { package: owner, definition: account };
}

import { TagMap } from "@destack/object";
import { PermissionReference } from "@destack/access/declare";
import { DeclarationName, ModuleMetadata, type Package } from "@destack/package";
import { defineSchema, Instant, schema } from "@destack/schema";
import { PackageError } from "@destack/package/error";

/** An environment an account defines. */
export const EnvironmentDefinition = defineSchema(
    schema.object({
        /** The environment's metadata tags. */
        tags: TagMap.exactOptional(),
    }),
);

/** An account role declared in source. */
export const AccountRole = defineSchema(
    schema.object({
        /** The role's purpose. */
        description: schema.string().min(1),
        /** The permissions the role grants wherever it is bound. */
        permissions: schema.array(PermissionReference),
    }),
);

/** A role granted to a user, a group or an installation in one of the account's spaces. */
export const AccountRoleBinding = defineSchema(
    schema.object({
        /** The declared role name or an existing role of this account. */
        role: schema.union([DeclarationName, schema.object({ id: schema.identifier("role") })]),
        /** The user, group or installation receiving the role. */
        subject: schema.union([
            schema.object({ user: schema.identifier("user") }),
            schema.object({ group: schema.identifier("group") }),
            schema.object({
                installation: schema.object({
                    /** The space of the installation. */
                    spaceId: schema.identifier("space"),
                    /** The installation. */
                    id: schema.identifier("installation"),
                }),
            }),
        ]),
        /** The space the role is bound on. */
        spaceId: schema.identifier("space").exactOptional(),
        /** The expiry, in UTC epoch milliseconds. */
        expiresAt: Instant.exactOptional(),
    }),
);

/** The account records a repository export declares. */
export const AccountDefinition = defineSchema(
    schema.object({
        /** The account's environments by name. */
        environments: schema.record(DeclarationName, EnvironmentDefinition).exactOptional(),
        /** The account roles that bindings refer to. */
        roles: schema.record(DeclarationName, AccountRole).exactOptional(),
        /** The role bindings, applied with the caller's existing authority. */
        bindings: schema.record(DeclarationName, AccountRoleBinding).exactOptional(),
    }),
);

/** The account records a repository export declares. */
export type AccountDefinition = schema.Infer<typeof AccountDefinition>;

/** The account records and the package that declares them. */
export interface AccountDeclaration {
    /** The declaring package, supplied by the module transform. */
    readonly package: Package;
    /** The declared records. */
    readonly definition: AccountDefinition;
}

/** Declare account records as data for a later apply. */
export function defineAccount(
    definition: schema.Input<typeof AccountDefinition>,
    module?: ModuleMetadata,
): AccountDeclaration {
    // stamp the declaring package and validate the records
    const owner = ModuleMetadata.require(module, "defineAccount").package;
    const account = AccountDefinition.parse(definition);

    // require each role a binding refers to by name in the same declaration
    for (const binding of Object.values(account.bindings ?? {})) {
        if (typeof binding.role === "string" && !Object.hasOwn(account.roles ?? {}, binding.role)) {
            throw new PackageError("INVALID_DEFINITION", `unknown role: ${binding.role}`);
        }
    }

    return { package: owner, definition: account };
}

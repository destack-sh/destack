import {
    check,
    type Column,
    foreignKey,
    type Identifier,
    identifier,
    integer,
    primaryKey,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { resource } from "./resource.ts";
import { installation } from "../space/installation.ts";
import { deployment } from "../space/deployment.ts";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";

/** A vault resource administered in a space. */
export const vault = table("vault", {
    /** The underlying resource. */
    resourceId: identifier("resource_id", "resource").primaryKey().notNull(),
    /** The resource's space. */
    spaceId: identifier("space_id", "space").notNull(),
    /** The fixed resource kind. */
    kind: text("kind").notNull().default("vault"),
}, (entry) => [
    unique("vault_space_id").on(entry.spaceId, entry.resourceId),
    foreignKey({
        columns: [entry.spaceId, entry.resourceId, entry.kind],
        foreignColumns: [resource.spaceId, resource.id, resource.kind],
    }).onDelete("restrict"),
    check("vault_kind", sql`${entry.kind} = 'vault'`),
]);

/** A named secret with independently versioned encrypted values. */
export const secret = table("secret", {
    ...recordColumns("secret"),
    ...provenanceColumns(),
    /** The administering space. */
    spaceId: identifier("space_id", "space").notNull(),
    /** The vault containing the secret. */
    vaultId: identifier("vault_id", "resource").notNull(),
    /** The vault-local name. */
    name: text("name").notNull(),
    /** The selected current version; null before a value is assigned. */
    currentVersion: integer("current_version"),
    /** Explicit revocation time. */
    revokedAt: integer("revoked_at"),
}, (entry) => [
    unique("secret_vault_name").on(entry.vaultId, entry.name),
    ...provenanceChecks("secret", entry),
    unique("secret_space_id").on(entry.spaceId, entry.id),
    foreignKey({
        columns: [entry.spaceId, entry.vaultId],
        foreignColumns: [vault.spaceId, vault.resourceId],
    }).onDelete("restrict"),
    foreignKey({
        columns: [entry.id, entry.currentVersion],
        foreignColumns: [secretVersion.secretId, secretVersion.version],
    }).onDelete("restrict"),
    check("secret_name", sql`length(${entry.name}) > 0`),
]);

/** A version stored by the vault backend, without its secret value. */
export const secretVersion = table("secret_version", {
    /** The owning secret. */
    secretId: identifier("secret_id", "secret").notNull().references(
        (): Column<Identifier<"secret">> => secret.id,
        {
            onDelete: "restrict",
        },
    ),
    /** The monotonically increasing value version. */
    version: integer("version").notNull(),
    /** Creation time in UTC epoch milliseconds. */
    createdAt: integer("created_at").notNull(),
    /** The backend's immutable version reference, retained after destruction. */
    reference: text("reference").notNull(),
    /** Optional value expiry. */
    expiresAt: integer("expires_at"),
    /** Explicit revocation time. */
    revokedAt: integer("revoked_at"),
    /** Time at which the encrypted value was destroyed. */
    destroyedAt: integer("destroyed_at"),
}, (entry) => [
    primaryKey({ columns: [entry.secretId, entry.version] }),
    check("secret_version_positive", sql`${entry.version} > 0`),
    check(
        "secret_version_reference",
        sql`length(${entry.reference}) > 0`,
    ),
    check(
        "secret_version_expiry",
        sql`${entry.expiresAt} IS NULL OR ${entry.expiresAt} > ${entry.createdAt}`,
    ),
]);

/** A secret selected for one package-local declaration. */
export const secretBinding = table("secret_binding", {
    /** The space containing the installation and secret. */
    spaceId: identifier("space_id", "space").notNull(),
    /** The installation receiving the binding. */
    installationId: identifier("installation_id", "installation").notNull(),
    /** The package declaring this secret. */
    packageId: identifier("package_id", "package").notNull(),
    /** The package-local secret declaration. */
    name: text("name").notNull(),
    /** The selected secret. */
    secretId: identifier("secret_id", "secret").notNull(),
    /** An exact version; null follows the current version at access time. */
    version: integer("version"),
}, (entry) => [
    primaryKey({ columns: [entry.installationId, entry.packageId, entry.name] }),
    foreignKey({
        columns: [entry.spaceId, entry.installationId],
        foreignColumns: [installation.spaceId, installation.id],
    }).onDelete("restrict"),
    foreignKey({
        columns: [entry.spaceId, entry.secretId],
        foreignColumns: [secret.spaceId, secret.id],
    }).onDelete("restrict"),
    foreignKey({
        columns: [entry.secretId, entry.version],
        foreignColumns: [secretVersion.secretId, secretVersion.version],
    }).onDelete("restrict"),
    check("secret_binding_name", sql`length(${entry.name}) > 0`),
]);

/** A secret selection retained with a deployment. */
export const deploymentSecretBinding = table("deployment_secret_binding", {
    /** The space containing the deployment and secret. */
    spaceId: identifier("space_id", "space").notNull(),
    /** The prepared deployment. */
    deploymentId: identifier("deployment_id", "deployment").notNull(),
    /** The package declaring this secret. */
    packageId: identifier("package_id", "package").notNull(),
    /** The package-local secret declaration. */
    name: text("name").notNull(),
    /** The selected secret. */
    secretId: identifier("secret_id", "secret").notNull(),
    /** An exact version; null preserves current-version selection. */
    version: integer("version"),
}, (entry) => [
    primaryKey({ columns: [entry.deploymentId, entry.packageId, entry.name] }),
    foreignKey({
        columns: [entry.spaceId, entry.deploymentId],
        foreignColumns: [deployment.spaceId, deployment.id],
    }).onDelete("restrict"),
    foreignKey({
        columns: [entry.spaceId, entry.secretId],
        foreignColumns: [secret.spaceId, secret.id],
    }).onDelete("restrict"),
    foreignKey({
        columns: [entry.secretId, entry.version],
        foreignColumns: [secretVersion.secretId, secretVersion.version],
    }).onDelete("restrict"),
    check("deployment_secret_binding_name", sql`length(${entry.name}) > 0`),
]);

/** A persisted vault. */
export type Vault = Select<typeof vault>;
/** A persisted secret. */
export type Secret = Select<typeof secret>;
/** A persisted secret version reference. */
export type SecretVersion = Select<typeof secretVersion>;
/** An installation secret selection. */
export type SecretBinding = Select<typeof secretBinding>;
/** A deployment secret selection. */
export type DeploymentSecretBinding = Select<typeof deploymentSecretBinding>;

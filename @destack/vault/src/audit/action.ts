import { defineAuditAction } from "@destack/audit";
import { Package } from "@destack/package";
import { schema, identifier } from "@destack/schema";
import definition from "../../destack.json" with { type: "json" };
import metadata from "../../package.json" with { type: "json" };

/** Package identifying all vault access and lifecycle actions. */
export const vaultPackage = Package.parse({
    id: definition.id,
    name: metadata.name,
    version: metadata.version,
});

/** Affected space and immutable secret identity. */
const secretTarget = schema.object({
    space: schema.object({ type: schema.literal("space"), id: identifier("space") }),
    secret: schema.object({ type: schema.literal("secret"), id: identifier("secret") }),
});
/** Optional immutable version selected by a secret operation. */
const secretDetails = schema.object({
    version: schema.number().int().positive().optional(),
});

/** Record secret.create actions. */
export const secretCreate = defineAuditAction({
    package: vaultPackage,
    name: "secret.create",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.update actions. */
export const secretUpdate = defineAuditAction({
    package: vaultPackage,
    name: "secret.update",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.disable actions. */
export const secretDisable = defineAuditAction({
    package: vaultPackage,
    name: "secret.disable",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.enable actions. */
export const secretEnable = defineAuditAction({
    package: vaultPackage,
    name: "secret.enable",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.delete actions. */
export const secretDelete = defineAuditAction({
    package: vaultPackage,
    name: "secret.delete",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.restore actions. */
export const secretRestore = defineAuditAction({
    package: vaultPackage,
    name: "secret.restore",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.purge actions. */
export const secretPurge = defineAuditAction({
    package: vaultPackage,
    name: "secret.purge",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.write actions. */
export const versionWrite = defineAuditAction({
    package: vaultPackage,
    name: "version.write",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.read actions. */
export const versionRead = defineAuditAction({
    package: vaultPackage,
    name: "version.read",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.promote actions. */
export const versionPromote = defineAuditAction({
    package: vaultPackage,
    name: "version.promote",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.disable actions. */
export const versionDisable = defineAuditAction({
    package: vaultPackage,
    name: "version.disable",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.enable actions. */
export const versionEnable = defineAuditAction({
    package: vaultPackage,
    name: "version.enable",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.destroy actions. */
export const versionDestroy = defineAuditAction({
    package: vaultPackage,
    name: "version.destroy",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.rewrap actions. */
export const versionRewrap = defineAuditAction({
    package: vaultPackage,
    name: "version.rewrap",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Declared secret actions indexed by their service operation. */
export const secretAction = {
    "secret.create": secretCreate,
    "secret.update": secretUpdate,
    "secret.disable": secretDisable,
    "secret.enable": secretEnable,
    "secret.delete": secretDelete,
    "secret.restore": secretRestore,
    "secret.purge": secretPurge,
    "version.write": versionWrite,
    "version.read": versionRead,
    "version.promote": versionPromote,
    "version.disable": versionDisable,
    "version.enable": versionEnable,
    "version.destroy": versionDestroy,
    "version.rewrap": versionRewrap,
};

/** Root-key replacement for persisted retry records. */
export const requestRewrap = defineAuditAction({
    package: vaultPackage,
    name: "request.rewrap",
    version: 1,
    targets: schema.object({
        space: schema.object({ type: schema.literal("space"), id: identifier("space") }),
    }),
    details: schema.object({
        previousKeyId: schema.string(),
        keyId: schema.string(),
        count: schema.number().int().positive(),
    }),
});

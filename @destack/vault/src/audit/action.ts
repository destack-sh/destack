import { defineAuditAction } from "@destack/audit";
import { schema, identifier } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** Package identifying all vault access and lifecycle actions. */
export const vaultPackage = import.meta.destack.package;

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
    name: "Secret.create",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.update actions. */
export const secretUpdate = defineAuditAction({
    name: "Secret.update",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.disable actions. */
export const secretDisable = defineAuditAction({
    name: "Secret.disable",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.enable actions. */
export const secretEnable = defineAuditAction({
    name: "Secret.enable",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.delete actions. */
export const secretDelete = defineAuditAction({
    name: "Secret.delete",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.restore actions. */
export const secretRestore = defineAuditAction({
    name: "Secret.restore",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record secret.purge actions. */
export const secretPurge = defineAuditAction({
    name: "Secret.purge",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.write actions. */
export const versionWrite = defineAuditAction({
    name: "Version.write",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.read actions. */
export const versionRead = defineAuditAction({
    name: "Version.read",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.promote actions. */
export const versionPromote = defineAuditAction({
    name: "Version.promote",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.disable actions. */
export const versionDisable = defineAuditAction({
    name: "Version.disable",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.enable actions. */
export const versionEnable = defineAuditAction({
    name: "Version.enable",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.destroy actions. */
export const versionDestroy = defineAuditAction({
    name: "Version.destroy",
    version: 1,
    targets: secretTarget,
    details: secretDetails,
});

/** Record version.rewrap actions. */
export const versionRewrap = defineAuditAction({
    name: "Version.rewrap",
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
    name: "Request.rewrap",
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

import { defineAuditAction } from "@destack/audit";
import { schema } from "@destack/schema";
import manifest from "../../package.json" with { type: "json" };
import definition from "../../destack.json" with { type: "json" };
import { PackageId } from "@destack/package";
import { PermissionReference } from "@destack/access";

/** The package producing daemon audit events. */
export const daemonPackage = {
    id: PackageId.parse(definition.id),
    name: manifest.name,
    version: manifest.version,
};

/** Change the local host's display name. */
export const renameHost = defineAuditAction({
    package: daemonPackage,
    name: "host.rename",
    version: 1,
    targets: schema.object({
        host: schema.object({ type: schema.literal("host"), id: schema.string().min(1) }),
    }),
    details: schema.object({ name: schema.string().min(1).max(128) }),
});

/** Register an existing working directory. */
export const registerCheckout = defineAuditAction({
    package: daemonPackage,
    name: "checkout.register",
    version: 1,
    targets: schema.object({
        checkout: schema.object({ type: schema.literal("checkout"), id: schema.string().min(1) }),
    }),
    details: schema.object({
        directory: schema.string(),
        repositoryId: schema.string().nullable(),
    }),
});

/** Remove a working directory registration. */
export const unregisterCheckout = defineAuditAction({
    package: daemonPackage,
    name: "checkout.unregister",
    version: 1,
    targets: schema.object({
        checkout: schema.object({ type: schema.literal("checkout"), id: schema.string().min(1) }),
    }),
    details: schema.object({
        directory: schema.string(),
        repositoryId: schema.string().nullable(),
    }),
});

/** Issue a restricted local application credential. */
export const createCredential = defineAuditAction({
    package: daemonPackage,
    name: "credential.create",
    version: 1,
    targets: schema.object({
        credential: schema.object({ type: schema.literal("credential"), id: schema.string() }),
    }),
    details: schema.object({
        packageId: PackageId,
        permissions: schema.array(PermissionReference),
    }),
});

/** Revoke a restricted local application credential. */
export const revokeCredential = defineAuditAction({
    package: daemonPackage,
    name: "credential.revoke",
    version: 1,
    targets: createCredential.targets,
    details: schema.object({}),
});

/** Start the daemon's authenticated local services. */
export const startDaemon = defineAuditAction({
    package: daemonPackage,
    name: "daemon.start",
    version: 1,
    targets: renameHost.targets,
    details: schema.object({ pid: schema.number().int().positive(), version: schema.string() }),
});

/** Stop the daemon's local services and release their resources. */
export const stopDaemon = defineAuditAction({
    package: daemonPackage,
    name: "daemon.stop",
    version: 1,
    targets: startDaemon.targets,
    details: startDaemon.details,
});

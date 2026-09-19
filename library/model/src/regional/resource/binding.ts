import { foreignKey, identifier, index, primaryKey, type Select, table, text } from "@destack/db";
import { installation } from "../space/installation.ts";
import { resource } from "./resource.ts";

/** ResourceBinding records. */
export const resourceBinding = table("resource_binding", {
    /** The space containing both installation and resource. */
    spaceId: identifier("space_id", "space").notNull(),
    /** The installation declaring the requirement. */
    installationId: identifier("installation_id", "installation").notNull(),
    /** The package declaring this resource. */
    packageId: identifier("package_id", "package").notNull(),
    /** The declaration name within the package. */
    name: text("name").notNull(),
    /** The provisioned resource satisfying the declaration. */
    resourceId: identifier("resource_id", "resource").notNull(),
}, (resourceBinding) => [
    primaryKey({
        columns: [resourceBinding.installationId, resourceBinding.packageId, resourceBinding.name],
    }),
    foreignKey({
        columns: [resourceBinding.spaceId, resourceBinding.installationId],
        foreignColumns: [installation.spaceId, installation.id],
    }).onDelete("cascade"),
    foreignKey({
        columns: [resourceBinding.spaceId, resourceBinding.resourceId],
        foreignColumns: [resource.spaceId, resource.id],
    }).onDelete("restrict"),
    index("resource_binding_resource").on(resourceBinding.spaceId, resourceBinding.resourceId),
]);

/** A persisted resource binding record. */
export type ResourceBinding = Select<typeof resourceBinding>;

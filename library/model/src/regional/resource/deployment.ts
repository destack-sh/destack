import {
    check,
    foreignKey,
    identifier,
    integer,
    primaryKey,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";
import { deployment } from "../space/deployment.ts";
import { resource } from "./resource.ts";

/** Resource bindings captured for a particular deployment. */
export const deploymentBinding = table("deployment_binding", {
    /** The space containing the deployment and resource. */
    spaceId: identifier("space_id", "space").notNull(),
    /** The prepared deployment. */
    deploymentId: identifier("deployment_id", "deployment").notNull(),
    /** The package-local resource declaration. */
    name: text("name").notNull(),
    /** The provisioned resource satisfying the declaration. */
    resourceId: identifier("resource_id", "resource").notNull(),
    /** The resource generation against which compatibility was checked. */
    generation: integer("generation").notNull(),
}, (entry) => [
    primaryKey({ columns: [entry.deploymentId, entry.name] }),
    foreignKey({
        columns: [entry.spaceId, entry.deploymentId],
        foreignColumns: [deployment.spaceId, deployment.id],
    }).onDelete("restrict"),
    foreignKey({
        columns: [entry.spaceId, entry.resourceId],
        foreignColumns: [resource.spaceId, resource.id],
    }).onDelete("restrict"),
    check("deployment_binding_generation", sql`${entry.generation} > 0`),
]);

/** A resource binding retained with a deployment. */
export type DeploymentBinding = Select<typeof deploymentBinding>;

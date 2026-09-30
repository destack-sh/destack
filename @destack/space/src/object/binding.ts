import {
    and,
    check,
    eq,
    foreignKey,
    index,
    sql,
    unique,
    type DatabaseConnection,
    type Select,
} from "@destack/db";
import { defineObject, field, method, type ObjectType } from "@destack/object";
import { PackageId } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { SpaceBinding } from "../declare/resource.ts";
import { Deployment, deployment } from "./deployment.ts";
import { installation } from "./installation.ts";
import { space } from "./space.ts";

/** An installation's binding of a package declaration to a bindable object in the space. */
export const binding = defineObject({
    name: "binding",
    plural: "bindings",
    scope: space,
    declarable: { schema: SpaceBinding },
    fields: {
        /** The installation whose package declares the need. */
        installationId: field.reference<"installation">((): ObjectType => installation, {
            delete: "cascade",
        }),
        /** The package declaring the need. */
        packageId: field.string(PackageId),
        /** The declaration name within the package. */
        name: field.string(),
        /** The bound object's identifier with its type as prefix. */
        target: field.string(),
        /** An exact version or generation to run with; absence follows the target's current one at capture. */
        version: field.integer().optional(),
        /** The state the declaration requires of the target. */
        state: field.json(schema.record(schema.string(), schema.json())),
    },
    constraints: (entry) => [
        foreignKey({
            columns: [entry.scope, entry.managerInstallationId],
            foreignColumns: [installation.table.scope, installation.table.id],
        }),
        unique("binding_declaration").on(entry.installationId, entry.packageId, entry.name),
        foreignKey({
            columns: [entry.scope, entry.installationId],
            foreignColumns: [installation.table.scope, installation.table.id],
        }).onDelete("cascade"),
        index("binding_target").on(entry.scope, entry.target),
        check("binding_name", sql`length(${entry.name}) > 0`),
        check("binding_version", sql`${entry.version} IS NULL OR ${entry.version} > 0`),
    ],
    methods: {
        create: method.create(null, { isSystem: true }),
        update: method.update(null, { isSystem: true, fields: ["state"] }),
        delete: method.delete(null, { isSystem: true }),
    },
});

/** What a deployment runs with for one of its declarations: the bound object at the version or generation captured when it was created. */
export const capture = defineObject({
    name: "capture",
    plural: "captures",
    scope: space,
    fields: {
        /** The deployment running with the capture. */
        deploymentId: field.reference<"deployment">((): ObjectType => deployment),
        /** The package declaring the need. */
        packageId: field.string(PackageId),
        /** The declaration name within the package. */
        name: field.string(),
        /** The bound object's identifier with its type as prefix. */
        target: field.string(),
        /** The target's version or generation the deployment runs with. */
        version: field.integer(),
        /** The state the deployment's declaration requires of the target, kept until the deployment retires. */
        state: field.json(schema.record(schema.string(), schema.json())),
    },
    constraints: (entry) => [
        unique("capture_declaration").on(entry.deploymentId, entry.packageId, entry.name),
        foreignKey({
            columns: [entry.scope, entry.deploymentId],
            foreignColumns: [deployment.table.scope, deployment.table.id],
        }).onDelete("restrict"),
        index("capture_target").on(entry.scope, entry.target),
        check("capture_version", sql`${entry.version} > 0`),
    ],
    permissions: ["read"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true }),
        delete: method.delete(null, { isSystem: true }),
    },
});

/** A persisted binding. */
export type Binding = Select<typeof binding.table>;

/** Checks of bindings that the packages serving bindable objects share. */
export const Binding = {
    /** Describe why a target is in use: an installation's binding targets it, or a live deployment captured it. */
    async inUse(
        database: DatabaseConnection,
        target: { readonly scope: Identifier<"space">; readonly id: string },
    ): Promise<string | undefined> {
        // find a binding targeting it
        const [bound] = await database
            .select({ installationId: binding.table.installationId })
            .from(binding.table)
            .where(and(eq(binding.table.scope, target.scope), eq(binding.table.target, target.id)))
            .limit(1);
        if (bound !== undefined) {
            return `bound by ${bound.installationId}`;
        }

        // find a live deployment running with it
        const [captured] = await database
            .select({ deploymentId: capture.table.deploymentId })
            .from(capture.table)
            .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
            .where(
                and(
                    eq(capture.table.scope, target.scope),
                    eq(capture.table.target, target.id),
                    Deployment.live(),
                ),
            )
            .limit(1);

        return captured === undefined ? undefined : `captured by ${captured.deploymentId}`;
    },

    /** Refuse removing a target while it is in use. */
    async requireUnbound(
        database: DatabaseConnection,
        target: { readonly scope: Identifier<"space">; readonly id: string },
    ): Promise<void> {
        const use = await Binding.inUse(database, target);
        if (use !== undefined) {
            throw new ServiceError("CONFLICT", { message: `${target.id} is in use: ${use}` });
        }
    },
};
/** A persisted capture. */
export type Capture = Select<typeof capture.table>;

/** Reads of captures that the packages serving deployments share. */
export const Capture = {
    /** Read the target a live deployment of an enabled installation captured for one of its declarations. */
    async target(
        database: DatabaseConnection,
        selector: {
            readonly spaceId: Identifier<"space">;
            readonly deploymentId: Identifier<"deployment">;
            readonly installationId: Identifier<"installation">;
            readonly packageId: PackageId;
            readonly name: string;
        },
    ): Promise<string | undefined> {
        const [captured] = await database
            .select({ target: capture.table.target })
            .from(capture.table)
            .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
            .innerJoin(
                installation.table,
                eq(installation.table.id, deployment.table.installationId),
            )
            .where(
                and(
                    eq(capture.table.deploymentId, selector.deploymentId),
                    eq(capture.table.scope, selector.spaceId),
                    eq(capture.table.packageId, selector.packageId),
                    eq(capture.table.name, selector.name),
                    eq(installation.table.id, selector.installationId),
                    eq(installation.table.status, "enabled"),
                    Deployment.live(),
                ),
            );

        return captured?.target;
    },
};

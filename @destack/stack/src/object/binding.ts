import { type Select, type Snapshot } from "@destack/db";
import { type ObjectOf, defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { Deployment, deployment } from "./deployment.ts";
import { installation } from "@destack/object";
import { space } from "@destack/account/object";
import type {} from "@destack/package/import-meta";
import { StackError } from "../error/index.ts";

/** The fields of a package declaration and its bound object, shared by bindings and captures. */
const boundFields = {
    /** The package declaring the need. */
    packageId: field.string(PackageId),
    /** The declaration name within the package. */
    name: field.string(),
    /** The bound object's id here, or the installation's name in the target space. */
    target: field.string(),
    /** The other space of the target installation, null for this space. */
    targetSpaceId: field.string(schema.identifier("space")).optional(),
    /** The state the declaration requires of the target, as a blob. */
    state: field.blob(),
};

/** An installation's binding of a package declaration to a bindable object in the space. */
export const binding = defineObject(
    {
        name: "binding",
        plural: "bindings",
        scope: space,
        fields: {
            /** The installation whose package declares the need. */
            installation: field.reference(
                (): ObjectOf<{ identity: "installation" }> => installation,
                {
                    delete: "cascade",
                },
            ),
            ...boundFields,
            /** An exact version or generation to run with, else the target's current one. */
            version: field.integer().optional(),
        },
        permissions: ["record"],
        constraints: {
            managerInstallation: {
                kind: "foreignKey",
                on: ["scope", "managerInstallationId"],
                references: { object: () => installation, on: ["scope", "id"] },
            },
            declaration: { kind: "unique", on: ["installationId", "packageId", "name"] },
            installation: {
                kind: "foreignKey",
                on: ["scope", "installationId"],
                references: { object: () => installation, on: ["scope", "id"] },
                onDelete: "cascade",
            },
            target: { kind: "index", on: ["scope", "target"] },
            name: { kind: "check", condition: { name: { length: { gt: 0 } } } },
            version: {
                kind: "check",
                condition: { version: { OR: [{ isNull: true }, { gt: 0 }] } },
            },
            targetSpace: {
                kind: "check",
                condition: {
                    OR: [
                        { targetSpaceId: { isNull: true } },
                        { targetSpaceId: { ne: { field: "scope" } }, version: { isNull: true } },
                    ],
                },
            },
        },
    },
    (method) => ({
        create: method.create("record", { visibility: "package" }),
        update: method.update("record", { visibility: "package", fields: ["state"] }),
        delete: method.delete("record", { visibility: "package" }),
        /** Delete a binding with the owned resource of its installation. */
        release: method.delete("record", { visibility: "package" }),
    }),
);

/** What a deployment runs with for one of its declarations. */
export const capture = defineObject(
    {
        name: "capture",
        plural: "captures",
        scope: space,
        fields: {
            /** The deployment running with the capture. */
            deployment: field.reference((): ObjectOf<{ identity: "deployment" }> => deployment),
            ...boundFields,
            /** The target's version or generation the deployment runs with. */
            version: field.integer().optional(),
        },
        permissions: ["read", "record"],
        constraints: {
            declaration: { kind: "unique", on: ["deploymentId", "packageId", "name"] },
            deployment: {
                kind: "foreignKey",
                on: ["scope", "deploymentId"],
                references: { object: () => deployment, on: ["scope", "id"] },
                onDelete: "no action",
            },
            target: { kind: "index", on: ["scope", "target"] },
            version: {
                kind: "check",
                condition: {
                    OR: [
                        { targetSpaceId: { isNull: true }, version: { gt: 0 } },
                        { targetSpaceId: { isNotNull: true }, version: { isNull: true } },
                    ],
                },
            },
        },
    },
    (method) => ({
        create: method.create("record", { visibility: "package" }),
        delete: method.delete("record", { visibility: "package" }),
    }),
);

/** A persisted binding. */
export type Binding = Select<typeof binding.table>;

/** Checks of bindings that the packages serving bindable objects share. */
export const Binding = {
    /** Key a binding or a resource need by its declaring package and declaration name. */
    key(packageId: string, name: string): string {
        return `${packageId} ${name}`;
    },

    /** Refuse destroying a target's version while a live deployment runs with it. */
    async requireReleased(
        reader: Pick<Snapshot, "read">,
        target: {
            readonly type: string;
            readonly scope: string;
            readonly id: string;
            readonly version: number;
        },
    ): Promise<void> {
        // leave a target outside spaces, which no deployment captures
        const parsed = schema.identifier("space").safeParse(target.scope);
        if (!parsed.success) {
            return;
        }

        // refuse a version a live deployment runs with
        const captured = await liveCapture(reader, parsed.data, target.id, target.version);
        if (captured !== undefined) {
            throw StackError.of(
                "VERSION_CAPTURED",
                `${target.type} version ${target.version} is captured by ${captured}`,
            );
        }
    },

    /** Refuse removing a target while a binding targets it or a live deployment captured it. */
    async requireUnbound(
        reader: Pick<Snapshot, "read">,
        target: { readonly scope: string; readonly id: string },
    ): Promise<void> {
        // leave a target outside spaces, which no installation binds
        const parsed = schema.identifier("space").safeParse(target.scope);
        if (!parsed.success) {
            return;
        }
        const scope = parsed.data;

        // refuse a target a binding targets
        const [bound] = await reader
            .read(binding.table)
            .findMany({ where: { scope, target: target.id } });
        if (bound !== undefined) {
            throw StackError.of(
                "BOUND",
                `${target.id} is in use: bound by ${bound.installationId}`,
            );
        }

        // refuse a target a live deployment runs with
        const captured = await liveCapture(reader, scope, target.id, undefined);
        if (captured !== undefined) {
            throw StackError.of("CAPTURED", `${target.id} is in use: captured by ${captured}`);
        }
    },
};

/** A persisted capture. */
export type Capture = Select<typeof capture.table>;

/** Read a live deployment capturing a target of a space, at a version when given. */
async function liveCapture(
    reader: Pick<Snapshot, "read">,
    scope: Identifier<"space">,
    id: string,
    version: number | undefined,
): Promise<Identifier<"deployment"> | undefined> {
    // read the captures of the target, and take one of a live deployment
    const captures = await reader.read(capture.table).findMany({
        where: {
            scope,
            target: id,
            ...(version === undefined ? {} : { version }),
        },
    });
    const [live] = await reader.read(deployment.table).findMany({
        where: {
            ...Deployment.live(),
            id: { in: captures.map((entry) => entry.deploymentId) },
        },
    });

    return live?.id;
}

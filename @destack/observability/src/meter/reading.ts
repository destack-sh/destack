import { principal } from "@destack/access";
import { bucket } from "@destack/stack/object";
import { and, type DatabaseConnection, eq, gt, isNotNull, isNull, lt, or } from "@destack/db";
import type { Meter } from "@destack/finance";
import { Duration, type Identifier, present, schema } from "@destack/schema";
import { database, deployment, instance } from "@destack/stack/object";
import { Subject } from "@destack/sync";
import { databaseBytes, memory, storageBytes } from "./meter.ts";

/** A machine's reading of one meter in a space. */
export interface Reading {
    /** The meter read. */
    readonly meter: Meter;
    /** The space measured. */
    readonly scope: Identifier<"space">;
    /** The installation using the resource. */
    readonly installation?: Identifier<"installation">;
    /** The measured value, in the meter's unit. */
    readonly value: number;
}

/** A machine's count of some work in a space since it last measured. */
export type Count = Omit<Reading, "meter">;

/** The readings a machine takes from its spaces' records. */
export const Reading = {
    /** Read each installation's GB-seconds in an interval from its instances' starts and stops. */
    async instances(
        records: DatabaseConnection,
        from: number,
        to: number,
        gigabytes: number,
    ): Promise<Reading[]> {
        // read the instances running at some point of the interval with their installations
        const rows = await records
            .select({
                scope: instance.table.scope,
                installation: deployment.table.installationId,
                startedAt: instance.table.startedAt,
                stoppedAt: instance.table.stoppedAt,
            })
            .from(instance.table)
            .innerJoin(deployment.table, eq(deployment.table.id, instance.table.deploymentId))
            .where(
                and(
                    isNotNull(instance.table.startedAt),
                    lt(instance.table.startedAt, to),
                    or(isNull(instance.table.stoppedAt), gt(instance.table.stoppedAt, from)),
                ),
            );

        // add up each installation's time inside the interval, as the memory held over it
        const byInstallation = Map.groupBy(rows, (row) => `${row.scope}\0${row.installation}`);

        return [...byInstallation.values()].map((run) => {
            // add up the run's milliseconds, then count them as seconds of the billed gigabytes
            const { scope, installation } = present(run[0], "an installation's instance");
            const milliseconds = run.reduce(
                (total, row) =>
                    total +
                    Math.min(to, row.stoppedAt ?? to) -
                    Math.max(from, present(row.startedAt, "a started instance's start")),
                0,
            );
            const value = Duration.seconds({ milliseconds }) * gigabytes;

            return { meter: memory, scope, installation, value };
        });
    },

    /** Read the bytes of each space's provisioned databases, by reference. */
    async databases(
        records: DatabaseConnection,
        bytes: (reference: string) => Promise<number>,
    ): Promise<Reading[]> {
        const rows = await records
            .select({
                scope: database.table.scope,
                reference: database.table.reference,
                owner: database.table.owner,
            })
            .from(database.table)
            .where(isNotNull(database.table.reference));

        return Promise.all(
            rows.map(async (row) => ({
                meter: databaseBytes,
                scope: row.scope,
                ...ownerOf(row.owner),
                value: await bytes(present(row.reference, "a provisioned database's reference")),
            })),
        );
    },

    /** Read the bytes of the object a machine keeps a space in, as the space's own databases. */
    async space(scope: Identifier<"space">, bytes: () => Promise<number>): Promise<Reading[]> {
        return [{ meter: databaseBytes, scope, value: await bytes() }];
    },

    /** Read the bytes of each installation's databases, by installation. */
    async installations(
        records: DatabaseConnection,
        bytes: (installed: {
            readonly scope: Identifier<"space">;
            readonly installationId: Identifier<"installation">;
        }) => Promise<number>,
    ): Promise<Reading[]> {
        // find the installations owning provisioned databases, each once
        const rows = await records
            .select({ scope: database.table.scope, owner: database.table.owner })
            .from(database.table)
            .where(isNotNull(database.table.reference))
            .groupBy(database.table.scope, database.table.owner);
        const owners = rows.flatMap((row) => {
            const { installation } = ownerOf(row.owner);

            return installation === undefined ? [] : [{ scope: row.scope, installation }];
        });

        // read each installation's bytes
        return Promise.all(
            owners.map(async ({ scope, installation }) => ({
                meter: databaseBytes,
                scope,
                installation,
                value: await bytes({ scope, installationId: installation }),
            })),
        );
    },

    /** Read a machine's counts on a meter, such as rows read or CPU milliseconds. */
    counted(meter: Meter, counts: readonly Count[]): Reading[] {
        return counts.map((count) => ({ ...count, meter }));
    },

    /** Read the bytes of each space's provisioned buckets, by bucket. */
    async buckets(
        records: DatabaseConnection,
        bytes: (bucketId: Identifier<"bucket">) => Promise<number>,
    ): Promise<Reading[]> {
        const rows = await records
            .select({ id: bucket.table.id, scope: bucket.table.scope, owner: bucket.table.owner })
            .from(bucket.table)
            .where(isNotNull(bucket.table.reference));

        return Promise.all(
            rows.map(async (row) => ({
                meter: storageBytes,
                scope: row.scope,
                ...ownerOf(row.owner),
                value: await bytes(row.id),
            })),
        );
    },
};

/** Read the installation owning a resource, none for one its space keeps itself. */
function ownerOf(owner: string | null): { readonly installation?: Identifier<"installation"> } {
    const subject = owner === null ? undefined : Subject.read(owner);

    return subject !== undefined && principal.installation.is(subject)
        ? { installation: schema.identifier("installation").parse(subject.id) }
        : {};
}

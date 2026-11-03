import { Provisioned, type ReaderPage } from "@destack/object";
import { and, type DatabaseConnection, eq, gt, isNotNull, isNull, lt, or } from "@destack/db";
import type { Meter } from "@destack/telemetry/meter";
import { Duration, type Identifier, present, schema } from "@destack/schema";
import { bucket, database, deployment, instance, reading } from "@destack/stack/object";
import { Event, EventTime } from "@destack/event";
import type { Crypto } from "@destack/environment";
import { RequestId } from "@destack/sync";
import { databaseBytes, memory, storageBytes } from "@destack/telemetry/meter";

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

        // add up each installation's time inside the interval, its memory constant over it
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
                ...Provisioned.ownerColumns(database.table),
            })
            .from(database.table)
            .where(isNotNull(database.table.reference));

        return Promise.all(
            rows.map(async (row) => ({
                meter: databaseBytes,
                scope: row.scope,
                ...ownerOf(row),
                value: await bytes(present(row.reference, "a provisioned database's reference")),
            })),
        );
    },

    /** Read the bytes of the object a machine stores a space in, as the space's own databases. */
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
            .select({ scope: database.table.scope, ...Provisioned.ownerColumns(database.table) })
            .from(database.table)
            .where(isNotNull(database.table.reference))
            .groupBy(
                database.table.scope,
                ...Object.values(Provisioned.ownerColumns(database.table)),
            );
        const owners = rows.flatMap((row) => {
            const { installation } = ownerOf(row);

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
            .select({
                id: bucket.table.id,
                scope: bucket.table.scope,
                ...Provisioned.ownerColumns(bucket.table),
            })
            .from(bucket.table)
            .where(isNotNull(bucket.table.reference));

        return Promise.all(
            rows.map(async (row) => ({
                meter: storageBytes,
                scope: row.scope,
                ...ownerOf(row),
                value: await bytes(row.id),
            })),
        );
    },

    /** Fold a page of a space's readings into each payer's uses, one per meter, SKU and month. */
    async uses(page: ReaderPage, crypto: Crypto) {
        // read each reading with the keys and data its kind declares
        const kind = reading.eventKind();
        const spaceId = schema.identifier("space").parse(page.key);
        const readings = page.events.map((event) => ({
            id: event.id,
            time: EventTime.milliseconds(event.time),
            keys: kind.parseKeys(event.keys),
            data: kind.parseData(event.data),
        }));

        // sum each payer's readings of a meter, SKU, installation and month
        const totals = Map.groupBy(readings, ({ time, keys }) =>
            JSON.stringify([
                keys.payerId,
                keys.meter,
                keys.sku,
                keys.installationId,
                monthStart(time),
            ]),
        );
        const uses = await Promise.all(
            [...totals.values()].map(async (summed) => {
                // span the readings' times, a level as its share of the month and an amount as read
                const first = present(summed[0], "a use's first reading");
                const last = present(summed.at(-1), "a use's last reading");
                const { payerId, meter, sku, installationId } = first.keys;
                const times = summed.map((each) => each.time);
                const from = Math.min(...times);
                const value = summed.reduce((total, each) => total + each.keys.value, 0);
                const { level } = last.data;
                const read = summed.map((each) => each.id).toSorted();
                const contents = [payerId, meter, sku, installationId, monthStart(from), ...read];

                return {
                    accountId: payerId,
                    read,
                    item: {
                        id: await Event.identity(spaceId, contents, crypto),
                        time: EventTime.of(from),
                        meter,
                        sku,
                        spaceId,
                        machineId: null,
                        installationId,
                        quantity: level === undefined ? value : value / monthLength(from),
                        level: level ?? null,
                        data: { from, to: Math.max(...times) },
                    },
                };
            }),
        );

        // gather each payer's uses in one request, keyed by its readings and timed by the claim, which a retry repeats
        return Promise.all(
            [...Map.groupBy(uses, (use) => use.accountId)].map(async ([accountId, payerUses]) => ({
                accountId,
                requestId: await RequestId.derive(
                    page.claimedAt,
                    JSON.stringify([accountId, ...payerUses.flatMap((use) => use.read).toSorted()]),
                    crypto,
                ),
                items: payerUses.map((use) => use.item),
            })),
        );
    },
};

/** Read the installation owning a resource as a reading has it, none for the space's own. */
function ownerOf(row: Parameters<typeof Provisioned.owner>[0]): {
    readonly installation?: Identifier<"installation">;
} {
    const installation = Provisioned.owner(row);

    return installation === undefined ? {} : { installation };
}

/** Read the start of the UTC month a time falls in, in Unix milliseconds. */
function monthStart(time: number): number {
    const date = new Date(time);

    return Date.UTC(date.getUTCFullYear(), date.getUTCMonth());
}

/** Read the length of the UTC month a time falls in, in milliseconds. */
function monthLength(time: number): number {
    const date = new Date(time);

    return Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + 1) - monthStart(time);
}

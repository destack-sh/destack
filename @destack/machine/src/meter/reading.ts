import { type CursorPage, Installation } from "@destack/object";
import {
    and,
    type DatabaseConnection,
    eq,
    gt,
    isNotNull,
    isNull,
    lt,
    or,
    Snapshot,
} from "@destack/db";
import type { Meter } from "@destack/telemetry/meter";
import { Duration, type Identifier, present, schema } from "@destack/schema";
import { bucket, database, deployment, instance, reading, usage } from "@destack/stack/object";
import { EventTime } from "@destack/event";
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
                manager: database.table.managerInstallationId,
            })
            .from(database.table)
            .where(isNotNull(database.table.reference));
        const applications = await managing(records, rows);

        return Promise.all(
            rows.map(async (row) => ({
                meter: databaseBytes,
                scope: row.scope,
                ...applicationOf(row, applications),
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
        // find the application installations managing provisioned databases, each once
        const rows = await records
            .select({ scope: database.table.scope, manager: database.table.managerInstallationId })
            .from(database.table)
            .where(isNotNull(database.table.reference))
            .groupBy(database.table.scope, database.table.managerInstallationId);
        const applications = await managing(records, rows);
        const owners = rows.flatMap((row) => {
            const { installation } = applicationOf(row, applications);

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
                manager: bucket.table.managerInstallationId,
            })
            .from(bucket.table)
            .where(isNotNull(bucket.table.reference));
        const applications = await managing(records, rows);

        return Promise.all(
            rows.map(async (row) => ({
                meter: storageBytes,
                scope: row.scope,
                ...applicationOf(row, applications),
                value: await bytes(row.id),
            })),
        );
    },

    /** Fold a page of a space's readings into its account's uses, one per meter, SKU and month. */
    async uses(page: CursorPage, crypto: Crypto) {
        // read each reading with the keys and data its kind declares
        const kind = reading.eventKind();
        const spaceId = schema.identifier("space").parse(page.scope);
        const readings = page.events.map((event) => ({
            id: event.id,
            time: EventTime.milliseconds(event.time),
            keys: kind.parseKeys(event.keys),
            data: kind.parseData(event.data),
        }));

        // sum the account's readings of each meter, SKU, installation and month
        const totals = Map.groupBy(readings, ({ time, keys }) =>
            JSON.stringify([
                keys.accountId,
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
                const { accountId, meter, sku, installationId } = first.keys;
                const times = summed.map((each) => each.time);
                const from = Math.min(...times);
                const value = summed.reduce((total, each) => total + each.keys.value, 0);
                const { level } = last.data;
                const read = summed.map((each) => each.id).toSorted();
                const contents = [accountId, meter, sku, installationId, monthStart(from), ...read];

                return {
                    accountId,
                    read,
                    item: {
                        id: await usage.derive(spaceId, JSON.stringify(contents), crypto),
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

        // gather each account's uses in one request, keyed by its readings and timed by the claim, which a retry repeats
        return Promise.all(
            [...Map.groupBy(uses, (use) => use.accountId)].map(
                async ([accountId, accountUses]) => ({
                    accountId,
                    requestId: await RequestId.derive(
                        page.claimedAt,
                        JSON.stringify([
                            accountId,
                            ...accountUses.flatMap((use) => use.read).toSorted(),
                        ]),
                        crypto,
                    ),
                    items: accountUses.map((use) => use.item),
                }),
            ),
        );
    },
};

/** Read which managers of some resources are application installations. */
function managing(
    records: DatabaseConnection,
    rows: readonly { readonly manager: Identifier<"installation"> | null }[],
): Promise<ReadonlySet<string>> {
    const managers = rows.flatMap((row) => (row.manager === null ? [] : [row.manager]));

    return Installation.applications(Snapshot.live(records), managers);
}

/** Read the application installation managing a resource, none for the space's own. */
function applicationOf(
    row: { readonly manager: Identifier<"installation"> | null },
    applications: ReadonlySet<string>,
): { readonly installation?: Identifier<"installation"> } {
    return row.manager !== null && applications.has(row.manager)
        ? { installation: row.manager }
        : {};
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

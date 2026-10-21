import { DeclarationReference } from "@destack/package/declare";
import { through, union } from "@destack/access";
import { account, entitlement, machine, space } from "@destack/account/object";
import { Table } from "@destack/db";
import { argument, plural, select, t } from "@destack/locale";
import { defineNotification } from "@destack/notification/declare";
import { Event, type EventKind, EventTime, type SERIES_FOLDS } from "@destack/event";
import {
    type MethodContext,
    defineObject,
    type EventOf,
    type EventView,
    field,
} from "@destack/object";
import type { Meter } from "@destack/observability/meter";
import type { Identifier } from "@destack/schema";
import type { Period } from "../rate/period.ts";
import { CallError, isCallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { present, schema } from "@destack/schema";
import type { ReaderPage } from "@destack/object";
import { reading } from "@destack/observability/object";
import { installation } from "@destack/stack/object";
import type { ReleaseReader } from "../release/reader.ts";

/** The share of a limit or budget at which usage is near it, where the soft cap first notifies. */
export const NEAR_SHARE = 0.8;

/** When usage flushes into a segment: hourly, or at 100,000 uses. */
const USAGE_FLUSH = { maxAge: { hours: 1 }, maxRows: 100_000 } as const;

/** How long the payer's home stores usage: ten years, as long as the invoices billing it. */
const USAGE_RETENTION = { days: 3650 } as const;

/** The keys a level is told apart by: each space, dedicated machine and installation. */
const LEVEL_KEYS = ["spaceId", "machineId", "installationId"] as const;

/** A fold a meter's usage is read with. */
type SeriesFold = (typeof SERIES_FOLDS)[number];

/** What a use has beside its keys. */
export const UsageData = schema.object({
    /** When the measure starts, in UTC epoch milliseconds. */
    from: schema.number().int(),
    /** When the measure ends, in UTC epoch milliseconds. */
    to: schema.number().int(),
});

/** A metered use of a resource, stored in the home of the account paying for it. */
export const usage = defineObject(
    {
        name: "usage",
        plural: "usages",
        scope: account,
        storage: {
            kind: "events",
            flush: USAGE_FLUSH,
            retention: USAGE_RETENTION,
            delivery: "exactly-once",
            isLocked: true,
        },
        // read each payer's usage from the log's start, stored thirty days past any rating outage
        readers: { usage: { by: "scope", start: "earliest", hold: { days: 30 } } },
        fields: {
            /** The measure's unit and interval. */
            data: field.json(UsageData),
            /** The meter measuring the use, by its package and name. */
            meter: field.string(),
            /** The SKU rating the use on the machine it ran on, absent for a seller's own meter. */
            sku: field.string().optional(),
            /** The space the use ran in, absent for a dedicated machine's own hours. */
            space: field.reference(space).optional(),
            /** The dedicated machine whose hours were used, absent for a space's use. */
            machine: field.reference(machine).optional(),
            /** The installation that used it, absent for the space itself. */
            installation: field.reference(installation).optional(),
            /** The amount used in the meter's unit, or the share of a month a level was held. */
            quantity: field.number(),
            /** The level held at the end of the measure, for meters averaged over time. */
            level: field.number().optional(),
        },
        permissions: {
            // let the payer's readers and its residency's servers read every use, a space's readers its own
            read: union(
                through("scope", "read"),
                through("scope", "serve"),
                through("space", "read"),
            ),
            // let a space, or a dedicated machine, append what it measured
            append: union(through("space", "serve"), through("machine", "serve")),
        },
    },
    (method) => ({
        /** Append measured uses. */
        append: method.createMany("append", {
            fields: [
                "meter",
                "sku",
                "spaceId",
                "machineId",
                "installationId",
                "quantity",
                "level",
                "data",
            ],
        }),
    }),
);

/** The series reads of the usage events, through a call or the store's event log. */
export type UsageSeries =
    ReturnType<typeof usage.eventKind> extends EventKind<infer Shape, infer Data>
        ? Pick<EventView<Shape, Data>, "series">
        : never;

/** A use of a resource as finance rates it. */
export type Usage = EventOf<typeof usage>;

/** An account's usage of a feature nears or meets its limit or its subscription's budget. */
export const usageLimit = defineNotification({
    name: "usageLimit",
    title: "Usage limits",
    description: "Your account's usage nears or meets its plan's limit or spending limit.",
    payload: Table.selectSchema(entitlement.table)
        .pick({ feature: true, state: true, limit: true })
        .extend({
            /** The usage of the feature since its last reset. */
            usage: schema.number().min(0),
        }),
    interruption: "timeSensitive",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: {
        title: t`${argument("feature")} ${select(argument("state"), {
            near: `at ${NEAR_SHARE * 100} percent`,
            over: "past the plan's included usage",
            other: "stopped at its limit",
        })}`,
        body: t`${select(argument("state"), {
            blocked:
                "Writes are refused and background work pauses until the plan changes or the next period starts. Reads and exports still work.",
            other: "Usage past the included usage bills at the listed unit prices.",
        })}`,
    },
    summary: t`${plural(argument("count"), { one: "# usage limit", other: "# usage limits" })}`,
});

/** The uses of resources finance rates. */
export const Usage = {
    /** Fold a page of a space's readings into each payer's uses, one per meter, SKU and month. */
    async of(page: ReaderPage) {
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
                        id: await Event.identity(spaceId, contents),
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
            [...Map.groupBy(uses, (use) => use.accountId)].map(async ([accountId, held]) => ({
                accountId,
                requestId: await RequestId.derive(
                    page.claimedAt,
                    JSON.stringify([accountId, ...held.flatMap((use) => use.read).toSorted()]),
                ),
                items: held.map((use) => use.item),
            })),
        );
    },

    /** Refuse a use its meter or SKU declaration rejects. */
    async require(used: Usage, releases: ReleaseReader): Promise<void> {
        // read the use's meter from its package's current release
        const { packageId, name } = DeclarationReference.ofKey(used.keys.meter);
        const { level, sku: skuKey } = used.keys;
        const meter = await releases.meter({ packageId, name });

        // require a level exactly for an averaged meter
        const isAveraged = meter.definition.aggregation === "average";
        if (isAveraged !== (level !== null)) {
            const verb = isAveraged ? "needs" : "refuses";
            throw new CallError("BAD_REQUEST", {
                message: `an event of meter ${name} ${verb} a level, as its aggregation is ${meter.definition.aggregation}`,
            });
        }

        // require the SKU to price the use's meter, whose usage adds up over a period
        if (skuKey === null) {
            return;
        }
        const reference = DeclarationReference.ofKey(skuKey);
        const sku = await releases.sku(reference);
        const measured = DeclarationReference.key(sku.definition.meter);
        if (measured !== DeclarationReference.key({ packageId, name })) {
            throw new CallError("BAD_REQUEST", {
                message: `sku ${sku.name} prices meter ${measured}, and the use measures ${name}`,
            });
        } else if (!isAveraged && meter.definition.aggregation !== "sum") {
            throw new CallError("BAD_REQUEST", {
                message: `sku ${sku.name} rates sum and average meters, and meter ${name} is ${meter.definition.aggregation}`,
            });
        }
    },

    /** Read the usage events' series through a call. */
    series(context: Pick<MethodContext, "events">): UsageSeries {
        return context.events(usage.eventKind());
    },

    /** Fold a paying account's usage of a meter from the events, in a period or now. */
    async read(
        payer: Identifier<"account">,
        meter: Meter,
        period: Period | undefined,
        now: number,
        events: UsageSeries,
    ): Promise<number> {
        // fold the meter's events in the range, each level apart by its source
        const key = DeclarationReference.key({ packageId: meter.package.id, name: meter.name });
        const from = period === undefined ? 0 : EventTime.of(period.start);
        const before = EventTime.of(period === undefined ? now + 1 : period.end);
        const { measure, fold, group } = Usage.request(meter.definition.aggregation, period);
        const folded = await events.series(
            {
                scopes: [payer],
                where: { AND: [{ time: { gte: from, lt: before } }, { meter: key }] },
            },
            { measure, fold, group },
        );

        // add up the groups and steps
        return folded
            .flatMap((entry) => entry.steps)
            .reduce((total, step) => total + step.value, 0);
    },

    /** Request the fold a meter's aggregation reads, each level apart by its source. */
    request(
        aggregation: Meter["definition"]["aggregation"],
        period: Period | undefined,
    ): {
        readonly measure: "quantity" | "level";
        readonly fold: SeriesFold;
        readonly group: readonly (typeof LEVEL_KEYS)[number][];
    } {
        // take the latest quantity of each space's installation
        if (aggregation === "last") {
            return { measure: "quantity", fold: "last", group: [...LEVEL_KEYS] };
        }
        // take the level each space's installation holds now, without a period to average over
        else if (aggregation === "average" && period === undefined) {
            return { measure: "level", fold: "last", group: [...LEVEL_KEYS] };
        }
        // add up the quantities of an averaged meter, to divide by the period
        else if (aggregation === "average") {
            return { measure: "quantity", fold: "sum", group: [] };
        }
        // fold the quantities as the aggregation does
        else {
            return { measure: "quantity", fold: aggregation, group: [] };
        }
    },

    /** Record the error of a use that never passes, rethrowing one that may pass on a retry. */
    refusal(error: unknown): CallError<string, unknown> {
        if (!isCallError(error) || error.code === "SERVICE_UNAVAILABLE") {
            throw error;
        }

        return error;
    },
};

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

import { DeclarationReference } from "@destack/package/declare";
import { FinanceError } from "../error/error.ts";
import { entitlement } from "@destack/account/object";
import { Table } from "@destack/db";
import { argument, plural, select, t } from "@destack/locale";
import { defineNotification } from "@destack/notification/declare";
import { type EventKind, EventTime, type SERIES_FOLDS } from "@destack/event";
import { type MethodContext, type EventOf, type EventView } from "@destack/object";
import { usage } from "@destack/stack/object";
import type { Meter } from "@destack/telemetry/meter";
import type { Identifier } from "@destack/schema";
import type { Period } from "../rate/period.ts";
import { CallError } from "@destack/object";
import { schema } from "@destack/schema";
import type { ReleaseReader } from "../release/reader.ts";
import type {} from "@destack/package/import-meta";

/** The share of a limit or budget at which usage is near it, where the soft cap first notifies. */
export const NEAR_SHARE = 0.8;

/** The keys a level is told apart by: each space, dedicated machine and installation. */
const LEVEL_KEYS = ["spaceId", "machineId", "installationId"] as const;

/** A fold a meter's usage is read with. */
type SeriesFold = (typeof SERIES_FOLDS)[number];

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
            throw FinanceError.of(
                "LEVEL_INVALID",
                `event of meter '${name}' is invalid: it '${verb}' a level because its aggregation is '${meter.definition.aggregation}'`,
            );
        }

        // require the SKU to price the use's meter, whose usage adds up over a period
        if (skuKey === null) {
            return;
        }
        const reference = DeclarationReference.ofKey(skuKey);
        const sku = await releases.sku(reference);
        const measured = DeclarationReference.key(sku.definition.meter);
        if (measured !== DeclarationReference.key({ packageId, name })) {
            throw FinanceError.of(
                "METER_INVALID",
                `meter '${name}' is invalid: sku '${sku.name}' prices meter '${measured}'`,
            );
        } else if (!meter.accepts(sku.definition.attributes)) {
            throw FinanceError.of(
                "METER_INVALID",
                `sku '${sku.name}' is invalid: it prices attributes ${JSON.stringify(sku.definition.attributes)} that meter '${name}' does not declare`,
            );
        } else if (!isAveraged && meter.definition.aggregation !== "sum") {
            throw FinanceError.of(
                "AGGREGATION_INVALID",
                `meter '${name}' is invalid: sku '${sku.name}' only rates sum and average meters, and it is '${meter.definition.aggregation}'`,
            );
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
        const known = CallError.of(error);
        if (known === undefined || known.code === "SERVICE_UNAVAILABLE") {
            throw error;
        }

        return known;
    },
};

import type { account, UsageState } from "@destack/account/object";
import type { Snapshot } from "@destack/db";
import type { IdentifierOf } from "@destack/object";
import type { Meter } from "@destack/observability/meter";
import type { PackageId } from "@destack/package";
import { CallError } from "@destack/runtime/error";
import type { ObjectReference } from "@destack/sync";
import type { Catalog } from "../catalog/catalog.ts";
import { FeatureLimit } from "../feature/feature.ts";
import type { FeatureGrant } from "../feature/grant.ts";
import { Period } from "../rate/period.ts";
import type { Allotment } from "./allotment.ts";
import { Charge } from "./charge.ts";
import { product } from "./product.ts";
import { productFeature } from "./product-feature.ts";
import { seller } from "./seller.ts";
import { Subscription, SubscriptionItem, subscriptionItem } from "./subscription.ts";
import { NEAR_SHARE, Usage, type UsageSeries } from "./usage.ts";

/** The usage cost a subscription allows in its period, and its cost so far, in minor units. */
interface Budget {
    /** The cost of the usage so far: its rated charges and its seller's metered prices. */
    readonly spend: number;
    /** The included usage, plus the spending limit when one is set. */
    readonly amount: number;
    /** Whether usage at the amount blocks further usage, as a spending limit does. */
    readonly isHard: boolean;
}

/** A subscription item, or a seller's default product, granting an account a product's features. */
export class Source {
    /** The granting subscription item or default product. */
    readonly reference: ObjectReference;
    /** The product's feature grants. */
    readonly grants: readonly FeatureGrant[];
    /** When the source was created. */
    readonly grantedAt: number;
    /** The period metered usage resets with. */
    readonly period: Period;
    /** What the subscription's usage may cost, absent for a default product. */
    readonly budget: Budget | undefined;

    /** Keep a source's grants, period and budget. */
    constructor(fields: Pick<Source, "reference" | "grants" | "grantedAt" | "period" | "budget">) {
        // keep the fields
        this.reference = fields.reference;
        this.grants = fields.grants;
        this.grantedAt = fields.grantedAt;
        this.period = fields.period;
        this.budget = fields.budget;
    }

    /** List an account's sources: granting subscription items and other sellers' defaults. */
    static async list(
        accountId: IdentifierOf<typeof account>,
        now: number,
        reader: Pick<Snapshot, "read">,
    ): Promise<Source[]> {
        // read each granting subscription's items with its period and budget
        const subscriptions = await Subscription.granting(accountId, reader);
        const sources: Source[] = [];
        for (const subscribed of subscriptions) {
            const budget = await readBudget(subscribed, reader);
            const period = {
                start: subscribed.currentPeriodStart,
                end: subscribed.currentPeriodEnd,
            };
            for (const item of subscribed.items) {
                const reference = subscriptionItem.reference(subscribed.scope, item.id);
                sources.push(
                    new Source({
                        reference,
                        grants: item.grants,
                        grantedAt: item.createdAt,
                        period,
                        budget,
                    }),
                );
            }
        }

        // read the default product of each other seller, its metered usage counted by calendar month
        const subscribed = new Set(subscriptions.map((row) => row.sellerId));
        const defaults = await reader
            .read(product.table)
            .findMany({ where: { isDefault: true, active: true } });
        const selling = await reader.read(seller.table).findMany({
            where: { accountId: { in: defaults.map((row) => row.accountId) } },
        });
        const sellers = new Map(selling.map((row) => [row.accountId, row.id]));
        const unsubscribed = defaults.filter((each) => {
            const sellerId = sellers.get(each.accountId);

            return sellerId === undefined || !subscribed.has(sellerId);
        });
        const features = Map.groupBy(
            await reader.read(productFeature.table).findMany({
                where: { parentId: { in: unsubscribed.map((row) => row.id) } },
            }),
            (row) => row.parentId,
        );
        for (const row of unsubscribed) {
            const grants = (features.get(row.id) ?? []).map(({ packageId, feature, value }) => ({
                packageId,
                feature,
                value,
            }));
            sources.push(
                new Source({
                    reference: product.reference(row.scope, row.id),
                    grants,
                    grantedAt: row.createdAt,
                    period: Period.month(now),
                    budget: undefined,
                }),
            );
        }

        return sources;
    }

    /** Allot one of the source's grants to an account, absent once its feature is gone. */
    async allot(
        accountId: IdentifierOf<typeof account>,
        grant: FeatureGrant,
        catalog: (packageId: PackageId) => Promise<Catalog>,
        events: UsageSeries,
        now: number,
    ): Promise<Allotment | undefined> {
        // read the feature's declaration, letting the grant lapse once its package removed it
        const reference = { packageId: grant.packageId, name: grant.feature };
        const definition = (await catalog(grant.packageId)).find(reference)?.definition;
        if (definition === undefined) {
            return undefined;
        }
        const granted = {
            packageId: grant.packageId,
            feature: grant.feature,
            kind: definition.kind,
            source: this.reference,
            grantedAt: this.grantedAt,
            period: this.period,
        };
        const unmetered = { limit: null, usage: null, resetAt: null, state: null };

        // grant access alone, or the product's fixed value
        if (definition.kind === "boolean") {
            return { ...granted, ...unmetered, value: null };
        } else if (definition.kind === "static") {
            return { ...granted, ...unmetered, value: grant.value };
        }

        // count the meters' usage within the source's period, or since the start without a reset
        const meters = await Promise.all(
            definition.meters.map(async (meter) => (await catalog(meter.packageId)).meter(meter)),
        );
        const period = definition.reset === "period" ? this.period : undefined;
        const usage = await readUsage(accountId, meters, period, events, now);
        const limit = grant.value === null ? null : FeatureLimit.parse(grant.value);

        return {
            ...granted,
            value: null,
            limit,
            usage,
            resetAt: period === undefined ? null : period.end,
            state: standing(usage, limit, this.budget),
        };
    }
}

/** Read a subscription's budget and spend so far. */
async function readBudget(
    row: Subscription & { readonly items: readonly SubscriptionItem[] },
    reader: Pick<Snapshot, "read">,
): Promise<Budget | undefined> {
    // add up the period's rated charges
    const spend = await Charge.spend(row, reader);

    // budget the included usage with the spending limit, or notify at the included usage alone
    const includedUsage = SubscriptionItem.includedUsage(row.items);
    if (row.spendingLimit !== null) {
        return { spend, amount: includedUsage + row.spendingLimit, isHard: true };
    } else if (includedUsage > 0) {
        return { spend, amount: includedUsage, isHard: false };
    }

    return undefined;
}

/** Decide where usage stands against a feature's blocking limit and its subscription budget. */
function standing(usage: number, limit: number | null, budget: Budget | undefined): UsageState {
    // compare usage with the limit, and the spend with the budget
    const caps = [
        ...(limit === null ? [] : [{ used: usage, amount: limit, isHard: true }]),
        ...(budget === undefined
            ? []
            : [{ used: budget.spend, amount: budget.amount, isHard: budget.isHard }]),
    ];

    // block at a hard cap, bill on past a soft one, and warn near either
    if (caps.some((cap) => cap.isHard && cap.used >= cap.amount)) {
        return "blocked";
    } else if (caps.some((cap) => cap.used >= cap.amount)) {
        return "over";
    } else if (caps.some((cap) => cap.used >= NEAR_SHARE * cap.amount)) {
        return "near";
    }

    return "within";
}

/** Add up a feature's meters' usage. */
async function readUsage(
    accountId: IdentifierOf<typeof account>,
    meters: readonly Meter[],
    period: Period | undefined,
    events: UsageSeries,
    now: number,
): Promise<number> {
    // require one unit, so the meters' usage adds up
    const units = new Set(meters.map((meter) => meter.definition.unit));
    if (units.size > 1) {
        throw new CallError("PRECONDITION_FAILED", {
            message: `a feature counts meters of several units: ${[...units].join(", ")}`,
        });
    }

    // add up each meter's usage
    let total = 0;
    for (const meter of meters) {
        total += await Usage.read(accountId, meter, period, now, events);
    }

    return total;
}

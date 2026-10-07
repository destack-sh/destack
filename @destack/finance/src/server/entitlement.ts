import { account, type UsageState } from "@destack/account/object";
import { and, Change, eq, inArray, type DatabaseConnection, type Table } from "@destack/db";
import type { Call, IdentifierOf } from "@destack/object";
import type { ObjectServer } from "@destack/object/server";
import type { Controller } from "@destack/service/control";
import { announcement } from "@destack/notification";
import { canonicalize, present } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { ServiceError } from "@destack/service/error";
import { FeatureLimit } from "../feature/feature.ts";
import type { MeterUsage } from "./usage.ts";
import { Period } from "../meter/usage.ts";
import type { Meter } from "../meter/meter.ts";
import {
    type AccountCall,
    Charge,
    type Entitlement,
    entitlement,
    type FeatureGrant,
    GRANTING_STATES,
    product,
    productFeature,
    subscription,
    type Subscription,
    type SubscriptionItem,
    subscriptionItem,
    usageLimit,
} from "../object/index.ts";
import type { PackageId } from "@destack/package";
import { Catalog } from "../catalog/catalog.ts";
import { asAccount } from "./account.ts";
import { deriveAllowances } from "./allowance.ts";

/** The usage states the account's readers of billing hear of. */
const NOTIFIED: ReadonlySet<UsageState | null> = new Set(["near", "over", "blocked"]);

/** The share of a limit or budget at which usage is near it, where the soft cap first notifies. */
const NEAR_SHARE = 0.8;

/** The identifier of an account. */
type AccountId = IdentifierOf<typeof account>;

/** An entitlement as one source derives it, before it is kept. */
type Draft = Pick<
    Entitlement,
    "packageId" | "feature" | "kind" | "value" | "limit" | "usage" | "resetAt" | "state" | "source"
>;

/** What a subscription's usage may cost in its period, and what it costs so far, in fractional minor units. */
interface Budget {
    /** The cost of the usage so far: its rated charges and its seller's metered prices. */
    readonly spend: number;
    /** The included usage, plus the spending limit when one is set. */
    readonly amount: number;
    /** Whether reaching the amount blocks usage, as a spending limit does, or only notifies. */
    readonly isHard: boolean;
}

/** A subscription item, or a seller's default product, granting a product's features. */
interface Source {
    /** The granting subscription item or default product. */
    readonly reference: ObjectReference;
    /** The product's feature grants. */
    readonly grants: readonly FeatureGrant[];
    /** The period metered usage resets with, and the subscription's budget. */
    readonly period: Period;
    /** What the subscription's usage may cost, absent for a default product. */
    readonly budget: Budget | undefined;
}

/** Derives each account's entitlements and allowances again as its subscriptions or any default product change. */
export class EntitlementController implements Controller {
    /** The controller's name in reports. */
    readonly name = "entitlement";
    /** The account copies, subscriptions and default products entitlements derive from. */
    readonly watches: readonly Table[] = [
        account.table,
        subscription.table,
        subscriptionItem.table,
        product.table,
        productFeature.table,
    ];
    /** The object server deriving as the system. */
    readonly #server: ObjectServer;
    /** Read the current catalogs declaring features and meters. */
    readonly #catalog: (packageId: PackageId) => Promise<Catalog>;
    /** The usage metered entitlements count. */
    readonly #meterUsage: MeterUsage;

    /** Derive through a server. */
    constructor(
        server: ObjectServer,
        catalog: (packageId: PackageId) => Promise<Catalog>,
        meterUsage: MeterUsage,
    ) {
        this.#server = server;
        this.#catalog = catalog;
        this.#meterUsage = meterUsage;
    }

    /** Select the account a copy or subscription changed, or every account as a product changes. */
    async keys(change: Change): Promise<readonly string[]> {
        if (Change.of(change, account.table)) {
            return [Change.image(change).id];
        } else if (Change.of(change, subscription.table)) {
            return [Change.image(change).scope];
        } else if (Change.of(change, subscriptionItem.table)) {
            return [Change.image(change).scope];
        }

        // NOTE #Performance: derive every account again as any product changes
        return this.list();
    }

    /** List every account copied here. */
    async list(): Promise<readonly string[]> {
        const rows = await this.#server.database
            .select({ id: account.table.id })
            .from(account.table);

        return rows.map((row) => row.id);
    }

    /** Derive an account's entitlements and allowances in one transaction. */
    async reconcile(scope: string): Promise<number | undefined> {
        await asAccount(this.#server, scope, (call) =>
            entitle(call, this.#catalog, this.#meterUsage),
        );

        return undefined;
    }
}

/** Derive an account's entitlements. */
export async function entitle(
    call: AccountCall,
    current: (packageId: PackageId) => Promise<Catalog>,
    meterUsage: MeterUsage,
): Promise<void> {
    // derive one entitlement per feature of each source's product
    const scope = account.identifier(call.scope);
    const catalog = Catalog.once(current);
    const derived = new Map<string, Draft>();
    for (const source of await readSources(call.database, scope, call.now)) {
        for (const grant of source.grants) {
            const draft = await derive(call.database, scope, source, grant, catalog, meterUsage);
            if (draft !== undefined) {
                derived.set(keyOf(draft), draft);
            }
        }
    }

    // read the kept entitlements
    const kept = await call.database
        .select()
        .from(entitlement.table)
        .where(eq(entitlement.table.scope, scope));

    // delete one no grant derives, and update a changed one
    for (const row of kept) {
        const wanted = derived.get(keyOf(row));
        if (wanted === undefined) {
            await call.invoke(entitlement).delete({ id: row.id });
        } else if (!isSame(row, wanted)) {
            const { value, limit, usage, resetAt, state } = wanted;
            await call
                .invoke(entitlement)
                .update({ id: row.id, value, limit, usage, resetAt, state });
        }
    }

    // create each derived entitlement not kept yet
    const existing = new Set(kept.map((row) => keyOf(row)));
    for (const [key, draft] of derived) {
        if (!existing.has(key)) {
            await call.invoke(entitlement).create(draft);
        }
    }
    // derive the allowances the account's cells and the account service enforce
    await deriveAllowances(call, call.scope, current);
}

/** Notify an entitlement's new usage state once per period. */
export async function notifyState(
    call: Call,
    row: Entitlement,
    previous: UsageState | null,
): Promise<void> {
    // leave a state the readers do not hear of, or one that did not change
    if (row.state === previous || !NOTIFIED.has(row.state)) {
        return;
    }

    // read the period of the granting subscription, or the calendar month of a default product
    const [held] =
        row.source.type === subscriptionItem.name
            ? await call.database
                  .select({ periodStart: subscription.table.currentPeriodStart })
                  .from(subscriptionItem.table)
                  .innerJoin(
                      subscription.table,
                      eq(subscription.table.id, subscriptionItem.table.parentId),
                  )
                  .where(eq(subscriptionItem.table.id, subscriptionItem.identifier(row.source.id)))
            : [{ periodStart: Period.month(call.now).start }];
    const { periodStart } = present(held, `subscription item ${row.source.id}`);

    // skip a state announced this period
    const key = `${row.state}:${periodStart}`;
    const [announced] = await call.database
        .select({ id: announcement.table.id })
        .from(announcement.table)
        .where(
            and(
                eq(announcement.table.packageId, usageLimit.reference.packageId),
                eq(announcement.table.name, usageLimit.name),
                eq(announcement.table.parentId, row.id),
                eq(announcement.table.key, key),
            ),
        );
    if (announced !== undefined) {
        return;
    }

    // announce it to the account's readers of billing
    await usageLimit.notify(call, {
        source: entitlement.reference(row.scope, row.id),
        key,
        thread: `${row.feature}:${periodStart}`,
        audience: { kind: "permission", permission: "read", reason: "subscribed" },
        payload: { feature: row.feature, state: row.state, usage: row.usage, limit: row.limit },
    });
}

/** Count rated usage into the metered entitlements. */
export async function measure(
    call: AccountCall,
    current: (packageId: PackageId) => Promise<Catalog>,
    meterUsage: MeterUsage,
): Promise<void> {
    // read the account's metered entitlements and their sources
    const scope = account.identifier(call.scope);
    const catalog = Catalog.once(current);
    const rows = await call.database
        .select()
        .from(entitlement.table)
        .where(and(eq(entitlement.table.scope, scope), eq(entitlement.table.kind, "metered")));
    const sources = new Map(
        (await readSources(call.database, scope, call.now)).map((source) => [
            source.reference.id,
            source,
        ]),
    );

    // read each one's usage and state again, skipping one whose source or declaration lapsed
    for (const row of rows) {
        const source = sources.get(row.source.id);
        const grant = { packageId: row.packageId, feature: row.feature, value: row.limit };
        const draft =
            source === undefined
                ? undefined
                : await derive(call.database, scope, source, grant, catalog, meterUsage);
        if (draft !== undefined && (draft.usage !== row.usage || draft.state !== row.state)) {
            await call
                .invoke(entitlement)
                .update({ id: row.id, usage: draft.usage, state: draft.state });
        }
    }
    // derive the allowances again from the counted usage
    await deriveAllowances(call, call.scope, current);
}

/** Read the account's sources: its granting subscriptions' items, and the default product of each seller it holds none with. */
async function readSources(
    database: DatabaseConnection,
    scope: AccountId,
    now: number,
): Promise<Source[]> {
    // read each granting subscription's items with its period and budget
    const subscriptions = await readSubscriptions(database, scope);
    const sources: Source[] = [];
    for (const subscribed of subscriptions) {
        const budget = await readBudget(database, subscribed);
        const period = { start: subscribed.currentPeriodStart, end: subscribed.currentPeriodEnd };
        for (const item of subscribed.items) {
            const reference = subscriptionItem.reference(scope, item.id);
            sources.push({ reference, grants: item.grants, period, budget });
        }
    }

    // read the default product of each other seller, its metered usage counted by calendar month
    const sellers = new Set(subscriptions.map((row) => row.seller.scope));
    const defaults = await database
        .select({ scope: product.table.scope, id: product.table.id })
        .from(product.table)
        .where(and(eq(product.table.isDefault, true), eq(product.table.active, true)));
    for (const row of defaults.filter((each) => !sellers.has(each.scope))) {
        const grants = await database
            .select({
                packageId: productFeature.table.packageId,
                feature: productFeature.table.feature,
                value: productFeature.table.value,
            })
            .from(productFeature.table)
            .where(eq(productFeature.table.parentId, row.id));
        const reference = product.reference(row.scope, row.id);
        sources.push({ reference, grants, period: Period.month(now), budget: undefined });
    }

    return sources;
}

/** Read the account's granting subscriptions with their items. */
async function readSubscriptions(
    database: DatabaseConnection,
    scope: AccountId,
): Promise<
    (Subscription & {
        readonly items: readonly Pick<SubscriptionItem, "id" | "quantity" | "terms" | "grants">[];
    })[]
> {
    // read the granting subscriptions
    const subscriptions = await database
        .select()
        .from(subscription.table)
        .where(
            and(
                eq(subscription.table.scope, scope),
                inArray(subscription.table.status, GRANTING_STATES),
            ),
        );
    if (subscriptions.length === 0) {
        return [];
    }

    // read their items
    const items = await database
        .select({
            id: subscriptionItem.table.id,
            parentId: subscriptionItem.table.parentId,
            quantity: subscriptionItem.table.quantity,
            terms: subscriptionItem.table.terms,
            grants: subscriptionItem.table.grants,
        })
        .from(subscriptionItem.table)
        .where(
            inArray(
                subscriptionItem.table.parentId,
                subscriptions.map((row) => row.id),
            ),
        );
    const byParent = Map.groupBy(items, (item) => item.parentId);

    return subscriptions.map((row) => ({ ...row, items: byParent.get(row.id) ?? [] }));
}

/** Read a subscription's budget and spend so far. */
async function readBudget(
    database: DatabaseConnection,
    row: Subscription & {
        readonly items: readonly Pick<SubscriptionItem, "id" | "quantity" | "terms" | "grants">[];
    },
): Promise<Budget | undefined> {
    // add up the period's rated charges
    const spend = await Charge.spend(database, row);

    // budget the included usage with the spending limit, or notify at the included usage alone
    const isMetered = row.items.some((item) => item.terms.recurring?.usage === "metered");
    if (row.providerId === null && !isMetered) {
        return undefined;
    } else if (row.spendingLimit !== null) {
        return { spend, amount: row.includedUsage + row.spendingLimit, isHard: true };
    } else if (row.includedUsage > 0) {
        return { spend, amount: row.includedUsage, isHard: false };
    }

    return undefined;
}

/** Derive the entitlement of one granted feature. */
async function derive(
    database: DatabaseConnection,
    scope: AccountId,
    source: Source,
    grant: FeatureGrant,
    catalog: (packageId: PackageId) => Promise<Catalog>,
    meterUsage: MeterUsage,
): Promise<Draft | undefined> {
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
        source: source.reference,
    };
    const unmetered = { limit: null, usage: null, resetAt: null, state: null };

    // grant access alone, or the product's fixed value
    if (definition.kind === "boolean") {
        return { ...granted, ...unmetered, value: null };
    } else if (definition.kind === "static") {
        return { ...granted, ...unmetered, value: grant.value };
    }

    // count the meters' usage within the subscription's period, or since the start without a reset
    const meters = await Promise.all(
        definition.meters.map(async (meter) => (await catalog(meter.packageId)).meter(meter)),
    );
    const period = definition.reset === "period" ? source.period : undefined;
    const usage = await readUsage(meterUsage, scope, meters, period, database);
    const limit = grant.value === null ? null : FeatureLimit.parse(grant.value);

    return {
        ...granted,
        value: null,
        limit,
        usage,
        resetAt: period === undefined ? null : period.end,
        state: standing(usage, limit, source.budget),
    };
}

/** Decide where usage stands against a feature's limit, which blocks, and its subscription's budget. */
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
    meterUsage: MeterUsage,
    scope: AccountId,
    meters: readonly Meter[],
    period: Period | undefined,
    database: DatabaseConnection,
): Promise<number> {
    // require one unit, so the meters' usage adds up
    const units = new Set(meters.map((meter) => meter.definition.unit));
    if (units.size > 1) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: `a feature counts meters of several units: ${[...units].join(", ")}`,
        });
    }

    // add up each meter's usage
    let usage = 0;
    for (const meter of meters) {
        usage += await meterUsage.read(scope, meter, period, database);
    }

    return usage;
}

/** Key an entitlement by its source, feature and kind. */
function keyOf(draft: Pick<Draft, "source" | "packageId" | "feature" | "kind">): string {
    return [draft.source.type, draft.source.id, draft.packageId, draft.feature, draft.kind].join(
        " ",
    );
}

/** Compare the derived fields of a kept entitlement. */
function isSame(row: Draft, wanted: Draft): boolean {
    return (
        canonicalize(row.value) === canonicalize(wanted.value) &&
        row.limit === wanted.limit &&
        row.usage === wanted.usage &&
        row.resetAt === wanted.resetAt &&
        row.state === wanted.state
    );
}

import { and, count, desc, eq, gte, inArray, lt, sql, type DatabaseConnection } from "@destack/db";
import { account } from "@destack/account/object";
import type { CallOf, IdentifierOf } from "@destack/object";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { aligned, canonicalize, found, present } from "@destack/schema";
import { FeatureLimit, type FeatureReset } from "../feature/feature.ts";
import { FeatureCatalog } from "../inspect/index.ts";
import { MeterReference, type Meter } from "../meter/meter.ts";
import {
    type customer,
    entitlement,
    type Entitlement,
    GRANTING_STATES,
    type FeatureGrant,
    meterEvent,
    type MeterEvent,
    purchase,
    subscription,
    subscriptionItem,
} from "../object/index.ts";

/** The identifier of an account. */
type AccountId = IdentifierOf<typeof account>;

/** Read a package's feature catalog. */
type ReadCatalog = (packageId: PackageId) => Promise<FeatureCatalog>;

/** Open the build of a package's current release, which declares its features and meters. */
export type OpenRelease = (packageId: PackageId) => Promise<BuildReader>;

/** An entitlement as one source derives it, before it is kept. */
type Draft = Pick<
    Entitlement,
    | "packageId"
    | "feature"
    | "kind"
    | "value"
    | "limit"
    | "usage"
    | "resetAt"
    | "source"
    | "sourceId"
>;

/** A billing period, in UTC epoch milliseconds from its start up to its end. */
interface Period {
    /** The first instant of the period. */
    readonly start: number;
    /** The first instant after the period. */
    readonly end: number;
}

/** A subscription or purchase granting the features of a price's product. */
interface Source {
    /** Whether a subscription or a purchase grants. */
    readonly kind: Entitlement["source"];
    /** The granting subscription item or purchase. */
    readonly id: string;
    /** The product's feature grants the source keeps. */
    readonly grants: readonly FeatureGrant[];
    /** The billing period of a subscription, absent for a purchase. */
    readonly period?: Period;
}

/** Derive an account's entitlements from its granting subscriptions and paid purchases, and their metered usage. */
export async function entitle(
    call: CallOf<typeof customer, "entitle">,
    release: OpenRelease,
): Promise<void> {
    // derive one entitlement per feature of each source's product
    const catalog = readCatalogs(release);
    const scope = account.identifier(call.scope);
    const derived = new Map<string, Draft>();
    for (const source of await readSources(call.database, scope)) {
        for (const grant of source.grants) {
            const draft = await derive(call.database, scope, source, grant, catalog);
            derived.set(keyOf(draft), draft);
        }
    }

    // read the kept entitlements
    const kept = await call.database
        .select()
        .from(entitlement.table)
        .where(eq(entitlement.table.scope, scope));

    // delete or update each kept entitlement
    for (const row of kept) {
        const wanted = derived.get(keyOf(row));

        // delete one no grant derives
        if (wanted === undefined) {
            await call.invoke(entitlement).delete({ id: row.id });
        }
        // update a changed one
        else if (!isSame(row, wanted)) {
            const { value, limit, usage, resetAt } = wanted;
            await call.invoke(entitlement).update({ id: row.id, value, limit, usage, resetAt });
        }
    }

    // create each derived entitlement not kept yet
    const existing = new Set(kept.map((row) => keyOf(row)));
    for (const [key, draft] of derived) {
        if (!existing.has(key)) {
            await call.invoke(entitlement).create(draft);
        }
    }
}

/** Measure a meter event into the usage of the account's entitlements on its meter, leaving other features as derived. */
export async function measure(
    call: CallOf<typeof meterEvent, "create">,
    event: MeterEvent,
    release: OpenRelease,
): Promise<void> {
    // read the account's metered entitlements
    const catalog = readCatalogs(release);
    const scope = account.identifier(call.scope);
    const rows = await call.database
        .select()
        .from(entitlement.table)
        .where(and(eq(entitlement.table.scope, scope), eq(entitlement.table.kind, "metered")));

    // select the entitlements counting the event's declared meter
    const counted = { packageId: event.packageId, name: event.meter };
    const meter = (await catalog(counted.packageId)).meter(counted);
    const counting: { readonly row: Entitlement; readonly reset: FeatureReset }[] = [];
    for (const row of rows) {
        const reference = { packageId: row.packageId, name: row.feature };
        const definition = (await catalog(row.packageId)).feature(reference).definition;
        if (
            definition.kind === "metered" &&
            MeterReference.key(definition.meter) === MeterReference.key(counted)
        ) {
            counting.push({ row, reset: definition.reset });
        }
    }

    // read each one's usage again when its subscription item's period holds the event
    for (const { row, reset } of counting) {
        const period =
            reset === "period" ? await readPeriod(call.database, row.sourceId) : undefined;
        const isHeld =
            period === undefined || (event.time >= period.start && event.time < period.end);
        const usage = isHeld ? await readUsage(call.database, scope, meter, period) : row.usage;
        if (usage !== row.usage) {
            await call.invoke(entitlement).update({ id: row.id, usage });
        }
    }
}

/** Read each package's catalog once per call. */
function readCatalogs(release: OpenRelease): ReadCatalog {
    const catalogs = new Map<PackageId, Promise<FeatureCatalog>>();

    return (packageId) => {
        const read =
            catalogs.get(packageId) ??
            release(packageId).then((reader) => FeatureCatalog.read(reader));
        catalogs.set(packageId, read);

        return read;
    };
}

/** Read the billing period of a subscription item's subscription. */
async function readPeriod(database: DatabaseConnection, itemId: string): Promise<Period> {
    const [period] = await database
        .select({
            start: subscription.table.currentPeriodStart,
            end: subscription.table.currentPeriodEnd,
        })
        .from(subscriptionItem.table)
        .innerJoin(subscription.table, eq(subscription.table.id, subscriptionItem.table.parentId))
        .where(eq(subscriptionItem.table.id, subscriptionItem.identifier(itemId)));

    return present(period, `the subscription of item ${itemId}`);
}

/** Read the account's sources: the items of its granting subscriptions, and its paid purchases. */
async function readSources(database: DatabaseConnection, scope: AccountId): Promise<Source[]> {
    // read the granting subscriptions
    const subscriptions = await database
        .select({
            id: subscription.table.id,
            start: subscription.table.currentPeriodStart,
            end: subscription.table.currentPeriodEnd,
        })
        .from(subscription.table)
        .where(
            and(
                eq(subscription.table.scope, scope),
                inArray(subscription.table.status, GRANTING_STATES),
            ),
        );

    // read their items, each in its subscription's period
    const periods = new Map(subscriptions.map(({ id, ...period }) => [id, period]));
    const items =
        subscriptions.length === 0
            ? []
            : await database
                  .select({
                      id: subscriptionItem.table.id,
                      parentId: subscriptionItem.table.parentId,
                      grants: subscriptionItem.table.grants,
                  })
                  .from(subscriptionItem.table)
                  .where(inArray(subscriptionItem.table.parentId, [...periods.keys()]));

    // read the paid purchases
    const purchases = await database
        .select({ id: purchase.table.id, grants: purchase.table.grants })
        .from(purchase.table)
        .where(and(eq(purchase.table.scope, scope), eq(purchase.table.status, "paid")));

    return [
        ...items.map((item) => ({
            kind: "subscription" as const,
            id: item.id,
            grants: item.grants,
            period: found(periods, item.parentId),
        })),
        ...purchases.map((row) => ({
            kind: "purchase" as const,
            id: row.id,
            grants: row.grants,
        })),
    ];
}

/** Derive the entitlement a source gives to one feature its product grants. */
async function derive(
    database: DatabaseConnection,
    scope: AccountId,
    source: Source,
    grant: FeatureGrant,
    catalog: ReadCatalog,
): Promise<Draft> {
    // read the feature's declaration
    const reference = { packageId: grant.packageId, name: grant.feature };
    const feature = (await catalog(grant.packageId)).feature(reference);
    const definition = feature.definition;
    const granted = {
        packageId: grant.packageId,
        feature: grant.feature,
        kind: definition.kind,
        source: source.kind,
        sourceId: source.id,
    };
    const unmetered = { limit: null, usage: null, resetAt: null };

    // grant access alone
    if (definition.kind === "boolean") {
        return { ...granted, ...unmetered, value: null };
    }
    // grant the product's fixed value
    else if (definition.kind === "static") {
        return { ...granted, ...unmetered, value: grant.value };
    }

    // refuse a metered grant of a purchase, which credit grants cover
    if (source.period === undefined) {
        throw new TypeError(`purchase ${source.id} grants metered feature ${grant.feature}`);
    }

    // count the meter's usage within the subscription's period, or since the start without a reset
    const meter = (await catalog(definition.meter.packageId)).meter(definition.meter);
    const period = definition.reset === "period" ? source.period : undefined;
    const usage = await readUsage(database, scope, meter, period);

    return {
        ...granted,
        value: null,
        limit: grant.value === null ? null : FeatureLimit.parse(grant.value),
        usage,
        resetAt: period === undefined ? null : period.end,
    };
}

/** Fold an account's events of a meter within a period, or all of them without one, by the meter's aggregation. */
async function readUsage(
    database: DatabaseConnection,
    scope: AccountId,
    meter: Meter,
    period: Period | undefined,
): Promise<number> {
    // select the meter's events of the account in the period
    const table = meterEvent.table;
    const where = and(
        eq(table.scope, scope),
        eq(table.packageId, meter.package.id),
        eq(table.meter, meter.name),
        period === undefined ? undefined : gte(table.time, period.start),
        period === undefined ? undefined : lt(table.time, period.end),
    );

    // take the latest value, zero without events
    const aggregation = meter.definition.aggregation;
    if (aggregation === "last") {
        const [latest] = await database
            .select({ value: table.value })
            .from(table)
            .where(where)
            .orderBy(desc(table.time), desc(table.id))
            .limit(1);

        return latest === undefined ? 0 : latest.value;
    }

    // fold the values, zero without events
    const folded = {
        sum: sql`coalesce(sum(${table.value}), 0)`,
        count: count(),
        max: sql`coalesce(max(${table.value}), 0)`,
    }[aggregation];
    const rows = await database.select({ usage: folded }).from(table).where(where);

    return Number(aligned(rows, 0).usage);
}

/** Key an entitlement by its source, feature and kind. */
function keyOf(draft: Draft): string {
    return [draft.source, draft.sourceId, draft.packageId, draft.feature, draft.kind].join(" ");
}

/** Compare the derived fields of a kept entitlement. */
function isSame(row: Draft, wanted: Draft): boolean {
    return (
        canonicalize(row.value) === canonicalize(wanted.value) &&
        row.limit === wanted.limit &&
        row.usage === wanted.usage &&
        row.resetAt === wanted.resetAt
    );
}

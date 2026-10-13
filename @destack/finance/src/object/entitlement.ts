import { through, union } from "@destack/access";
import {
    account,
    allowance,
    type AllowanceGrant,
    type AllowanceResource,
    USAGE_STATES,
    type UsageState,
    space,
} from "@destack/account/object";
import { type Select, type Snapshot } from "@destack/db";
import { type Message, t } from "@destack/locale";
import { announcement } from "@destack/notification";
import { defineNotification } from "@destack/notification/declare";
import {
    type Call,
    defineObject,
    field,
    type IdentifierOf,
    type ObjectCall,
} from "@destack/object";
import { PackageId } from "@destack/package";
import {
    aligned,
    canonicalize,
    Identifier,
    type JsonValue,
    present,
    schema,
    Text,
} from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { ObjectReference } from "@destack/sync";
import { Catalog } from "../catalog/catalog.ts";
import { CatalogName, CatalogReference } from "../catalog/reference.ts";
import { FEATURE_KINDS, FeatureLimit } from "../feature/feature.ts";
import type { Meter } from "../meter/meter.ts";
import { Period } from "../meter/usage.ts";
import type { Metering } from "../meter/metering.ts";
import { Charge } from "./charge.ts";
import { product } from "./product.ts";
import { type FeatureGrant } from "../feature/grant.ts";
import { productFeature } from "./product-feature.ts";
import { seller } from "./seller.ts";
import { subscription, Subscription, SubscriptionItem, subscriptionItem } from "./subscription.ts";

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

/** The usage cost a subscription allows in its period, and its cost so far, in minor units. */
interface Budget {
    /** The cost of the usage so far: its rated charges and its seller's metered prices. */
    readonly spend: number;
    /** The included usage, plus the spending limit when one is set. */
    readonly amount: number;
    /** Whether usage at the amount blocks further usage, as a spending limit does. */
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

/** The fields an allowance takes from the entitlements it is derived from. */
interface Derived {
    /** The resource allowed. */
    readonly resource: AllowanceResource;
    /** What the plan grants on it. */
    readonly grant: AllowanceGrant;
}

/** What an account may use of a feature through a subscription item or a default product. */
export const entitlement = defineObject({
    name: "entitlement",
    plural: "entitlements",
    scope: space,
    owner: "account",
    fields: {
        /** The account the row belongs to. */
        account: field.reference(account).relate("account"),
        /** The package declaring the feature. */
        packageId: field.string(PackageId),
        /** The feature's name in its package. */
        feature: field.string(CatalogName),
        /** What the feature grants. */
        kind: field.enum(FEATURE_KINDS),
        /** The value of a static feature. */
        value: field.json(schema.json()).optional(),
        /** The usage limit of a metered feature, unlimited without one. */
        limit: field.integer().optional(),
        /** The usage of a metered feature since its last reset. */
        usage: field.number().optional(),
        /** When the usage of a metered feature starts again, absent for usage that never resets. */
        resetAt: field.time().optional(),
        /** Where the usage of a metered feature stands against its limit and budget. */
        state: field.enum(USAGE_STATES).optional(),
        /** The subscription item or default product granting the feature. */
        source: field.json(ObjectReference),
    },
    permissions: {
        // the account's readers, and the servers of its residency, such as its cloud machines
        read: union(through("account", "read"), through("account", "serve")),
        // the servers of its residency derive it
        provide: through("account", "serve"),
    },
    attachments: [announcement.attach({ by: "read" })],
    constraints: {
        unique: {
            kind: "index",
            on: ["accountId", "source", "packageId", "feature"],
            unique: true,
        },
        kind: {
            kind: "check",
            condition: {
                AND: [
                    {
                        OR: [
                            { kind: "metered", usage: { isNotNull: true } },
                            { kind: { ne: "metered" }, usage: { isNull: true } },
                        ],
                    },
                    {
                        OR: [
                            { kind: "metered", state: { isNotNull: true } },
                            { kind: { ne: "metered" }, state: { isNull: true } },
                        ],
                    },
                    {
                        OR: [
                            { kind: "metered" },
                            { limit: { isNull: true }, resetAt: { isNull: true } },
                        ],
                    },
                    { OR: [{ kind: "static" }, { value: { isNull: true } }] },
                ],
            },
        },
    },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    create: method.create("provide", { isInternal: true }),
    update: method.update("provide", {
        isInternal: true,
        fields: ["value", "limit", "usage", "resetAt", "state"],
    }),
    delete: method.delete("provide", { isInternal: true }),
    derive: method.mutation({
        permission: "provide",
        target: "none",
        isInternal: true,
        input: schema.object({
            /** The account whose entitlements are derived. */
            accountId: schema.identifier("account"),
        }),
        output: schema.object({}),
    }),
}));
/** A persisted entitlement. */
export type Entitlement = Select<typeof entitlement.table>;

/** An account's usage of a feature nears or meets its limit or its subscription's budget. */
export const usageLimit = defineNotification({
    name: "usageLimit",
    title: "Usage limits",
    description: "Your account's usage nears or meets its plan's allowance or spending limit.",
    payload: entitlement.rowSchema().pick({ feature: true, state: true, usage: true, limit: true }),
    interruption: "timeSensitive",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: (payload) => ({
        title: titleOf(payload.feature, payload.state),
        body:
            payload.state === "blocked"
                ? t`Writes are refused and background work pauses until the plan changes or the next period starts. Reads and exports keep working.`
                : t`Usage past the included usage bills at the listed unit prices.`,
    }),
    summary: (count) => t`${count} usage limits`,
});

/** What an account may use of a feature across every source granting it. */
export type Resolution =
    | {
          /** Access alone. */
          readonly kind: "boolean";
      }
    | {
          /** A fixed value. */
          readonly kind: "static";
          /** The highest numeric value, or else the most recently granted one. */
          readonly value: JsonValue;
      }
    | {
          /** Usage of meters up to a limit. */
          readonly kind: "metered";
          /** The sum of the sources' limits, unlimited when any source is. */
          readonly limit: number | null;
          /** The usage within the current period of the source resetting first. */
          readonly usage: number;
          /** The reset of the source resetting first, none when no source resets. */
          readonly resetAt: number | null;
          /** The least advanced state of the sources, since any source with room allows usage. */
          readonly state: UsageState;
      };

/** The entitlements of accounts. */
export const Entitlement = {
    /** Resolve a feature's effective grant across an account's entitlements, if any. */
    resolve(rows: readonly Entitlement[], feature: CatalogReference): Resolution | null {
        // select the feature's rows and require one kind among them
        const granted = rows.filter(
            (row) => row.packageId === feature.packageId && row.feature === feature.name,
        );
        const [first] = granted;
        if (first === undefined) {
            return null;
        } else if (granted.some((row) => row.kind !== first.kind)) {
            throw new TypeError(`entitlements of ${feature.name} disagree on its kind`);
        }

        // grant access when any source does
        if (first.kind === "boolean") {
            return { kind: "boolean" };
        }
        // take the highest numeric value, or else the most recently granted one
        else if (first.kind === "static") {
            return { kind: "static", value: resolveValue(granted) };
        }

        // add the limits up, unlimited when any source is
        const limits = granted.flatMap((row) => (row.limit === null ? [] : [row.limit]));
        const limit =
            limits.length < granted.length ? null : limits.reduce((sum, each) => sum + each, 0);

        // count the usage within the period of the source resetting first
        const resetting = aligned(
            granted.toSorted(
                (left, right) => (left.resetAt ?? Infinity) - (right.resetAt ?? Infinity),
            ),
            0,
        );

        // take the least advanced state among the sources
        const states = granted.map((row) => USAGE_STATES.indexOf(present(row.state, "a state")));
        const state = present(USAGE_STATES[Math.min(...states)], "a usage state");

        return {
            kind: "metered",
            limit,
            usage: present(resetting.usage, "a metered usage"),
            resetAt: resetting.resetAt,
            state,
        };
    },
    /** Derive an account's entitlements from its sources' grants, and the allowances they grant. */
    async derive(call: Call, accountId: AccountId, metering: Metering): Promise<void> {
        // derive one entitlement per feature of each source's product
        const catalog = Catalog.once(metering.catalog);
        const derived = new Map<string, Draft>();
        const snapshot = call.view();
        for (const source of await readSources(snapshot, accountId, call.now)) {
            for (const grant of source.grants) {
                const draft = await draftOf(snapshot, accountId, source, grant, catalog, metering);
                if (draft !== undefined) {
                    derived.set(keyOf(draft), draft);
                }
            }
        }

        // read the kept entitlements
        const kept = await snapshot.rows(entitlement.table, { accountId });

        // delete one no grant derives, and update a changed one
        const changed = kept.flatMap((row): ObjectCall[] => {
            const wanted = derived.get(keyOf(row));
            if (wanted === undefined) {
                return [entitlement.call("delete", { id: row.id })];
            } else if (!isSame(row, wanted)) {
                const { value, limit, usage, resetAt, state } = wanted;

                return [
                    entitlement.call("update", { id: row.id, value, limit, usage, resetAt, state }),
                ];
            }

            return [];
        });

        // create each derived entitlement not kept yet
        const existing = new Set(kept.map((row) => keyOf(row)));
        const created = [...derived]
            .filter(([key]) => !existing.has(key))
            .map(([, draft]) => entitlement.call("create", { ...draft, accountId }));
        await call.mutate(call.service, [...changed, ...created]);

        // derive the allowances the account's machines and the account service enforce
        await allow(call, accountId, catalog);
    },

    /** Notify an entitlement's new usage state once per period. */
    async notify(call: Call, row: Entitlement, previous: UsageState | null): Promise<void> {
        // leave a state the readers do not hear of, or one that did not change
        if (row.state === previous || !NOTIFIED.has(row.state)) {
            return;
        }

        // read the period of the granting subscription, or the calendar month of a default product
        const periodStart =
            row.source.type === subscriptionItem.name
                ? await readPeriodStart(call.view(), row.source.id)
                : Period.month(call.now).start;

        // skip a state announced this period
        const key = `${row.state}:${periodStart}`;
        const [announced] = await call.read(announcement).rows({
            packageId: usageLimit.reference.packageId,
            name: usageLimit.name,
            parentId: row.id,
            key,
        });
        if (announced !== undefined) {
            return;
        }

        // announce it to the account's readers of billing
        await call.mutate(call.caller, [
            usageLimit.call({
                source: entitlement.reference(row.scope, row.id),
                key,
                thread: `${row.feature}:${periodStart}`,
                audience: { kind: "permission", permission: "read", reason: "subscribed" },
                payload: {
                    feature: row.feature,
                    state: row.state,
                    usage: row.usage,
                    limit: row.limit,
                },
            }),
        ]);
    },
};

/** Title a notification of the usage state a feature reached. */
function titleOf(feature: string, state: UsageState | null): Message {
    // tell a near state its share of the limit
    if (state === "near") {
        return t`${feature} at ${NEAR_SHARE * 100} percent`;
    }
    // tell an over state it bills past the included usage
    else if (state === "over") {
        return t`${feature} past the plan's included usage`;
    }

    return t`${feature} stopped at its limit`;
}

/** Resolve static entitlements: the highest number, or else the latest granted value. */
function resolveValue(granted: readonly Entitlement[]): JsonValue {
    // take the highest number when every value is one
    const values = granted.map((row) => present(row.value, "a static value"));
    const numbers = values.filter((value) => typeof value === "number");
    if (numbers.length === values.length) {
        return Math.max(...numbers);
    }

    // take the value of the latest source, by its time-ordered identifier on equal derivation times
    const latest = granted.toSorted(
        (left, right) =>
            right.createdAt - left.createdAt ||
            Text.compare(sourceOrder(right.source.id), sourceOrder(left.source.id)),
    );

    return present(aligned(latest, 0).value, "a static value");
}

/** Read the time-ordered part of a source's identifier, its UUIDv7 after its type's prefix. */
function sourceOrder(sourceId: string): string {
    return Identifier.uuid(schema.anyIdentifier().parse(sourceId));
}

/** Read the account's sources: granting subscription items and unsubscribed sellers' defaults. */
async function readSources(
    snapshot: Snapshot,
    accountId: AccountId,
    now: number,
): Promise<Source[]> {
    // read each granting subscription's items with its period and budget
    const subscriptions = await Subscription.granting(snapshot, accountId);
    const sources: Source[] = [];
    for (const subscribed of subscriptions) {
        const budget = await readBudget(snapshot, subscribed);
        const period = { start: subscribed.currentPeriodStart, end: subscribed.currentPeriodEnd };
        for (const item of subscribed.items) {
            const reference = subscriptionItem.reference(subscribed.scope, item.id);
            sources.push({ reference, grants: item.grants, period, budget });
        }
    }

    // read the default product of each other seller, its metered usage counted by calendar month
    const subscribed = new Set(subscriptions.map((row) => row.seller.id));
    const defaults = await snapshot.rows(product.table, { isDefault: true, active: true });
    const selling = await snapshot.rows(seller.table, {
        accountId: { in: defaults.map((row) => row.accountId) },
    });
    const sellers = new Map(selling.map((row) => [row.accountId, row.id]));
    const unsubscribed = defaults.filter((each) => {
        const sellerId = sellers.get(each.accountId);

        return sellerId === undefined || !subscribed.has(sellerId);
    });
    for (const row of unsubscribed) {
        const features = await snapshot.rows(productFeature.table, { parentId: row.id });
        const grants = features.map(({ packageId, feature, value }) => ({
            packageId,
            feature,
            value,
        }));
        const reference = product.reference(row.scope, row.id);
        sources.push({ reference, grants, period: Period.month(now), budget: undefined });
    }

    return sources;
}

/** Read a subscription's budget and spend so far. */
async function readBudget(
    snapshot: Snapshot,
    row: Subscription & { readonly items: readonly SubscriptionItem[] },
): Promise<Budget | undefined> {
    // add up the period's rated charges
    const spend = await Charge.spend(snapshot, row);

    // budget the included usage with the spending limit, or notify at the included usage alone
    const includedUsage = SubscriptionItem.includedUsage(row.items);
    if (row.spendingLimit !== null) {
        return { spend, amount: includedUsage + row.spendingLimit, isHard: true };
    } else if (includedUsage > 0) {
        return { spend, amount: includedUsage, isHard: false };
    }

    return undefined;
}

/** Derive the entitlement of one granted feature. */
async function draftOf(
    snapshot: Snapshot,
    accountId: AccountId,
    source: Source,
    grant: FeatureGrant,
    catalog: (packageId: PackageId) => Promise<Catalog>,
    metering: Metering,
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
    const usage = await readUsage(metering, accountId, meters, period);
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
    metering: Metering,
    accountId: AccountId,
    meters: readonly Meter[],
    period: Period | undefined,
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
        usage += await metering.read(accountId, meter, period);
    }

    return usage;
}

/** Key an entitlement by its source, feature and kind. */
function keyOf(draft: Pick<Draft, "source" | "packageId" | "feature" | "kind">): string {
    const feature = CatalogReference.key({ packageId: draft.packageId, name: draft.feature });

    return [draft.source.type, draft.source.id, feature, draft.kind].join(" ");
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

/** Read the start of the current period of the subscription an item belongs to. */
async function readPeriodStart(snapshot: Snapshot, itemId: string): Promise<number> {
    const item = await snapshot.row(subscriptionItem.table, {
        id: subscriptionItem.identifier(itemId),
    });
    const held =
        item === null ? null : await snapshot.row(subscription.table, { id: item.parentId });

    return present(held, `the subscription of item ${itemId}`).currentPeriodStart;
}

/** Derive an account's allowances from its entitlements. */
async function allow(
    call: Call,
    accountId: AccountId,
    catalog: (packageId: PackageId) => Promise<Catalog>,
): Promise<void> {
    // derive the wanted allowances, by resource
    const snapshot = call.view();
    const granted = await snapshot.rows(entitlement.table, { accountId });
    const wanted = await allowances(granted, catalog);

    // update each kept allowance that changed, and remove those no longer granted
    const kept = await snapshot.rows(allowance.table, { accountId });
    const calls = [];
    for (const row of kept) {
        const next = wanted.get(row.resource);
        if (next === undefined) {
            calls.push(allowance.call("delete", { id: row.id }));
        } else if (canonicalize(row.grant) !== canonicalize(next.grant)) {
            calls.push(allowance.call("update", { id: row.id, grant: next.grant }));
        }
        wanted.delete(row.resource);
    }

    // create the new ones, all in one mutation
    calls.push(
        ...[...wanted.values()].map((next) => allowance.call("create", { accountId, ...next })),
    );
    if (calls.length > 0) {
        await call.mutate(call.service, calls);
    }
}

/** Resolve the allowance of each granted feature declaring one. */
async function allowances(
    granted: readonly Entitlement[],
    catalog: (packageId: PackageId) => Promise<Catalog>,
): Promise<Map<AllowanceResource, Derived>> {
    // list each granted feature once
    const features = new Map(
        granted.map((row) => {
            const reference = { packageId: row.packageId, name: row.feature };

            return [CatalogReference.key(reference), reference];
        }),
    );

    // resolve the allowance of each feature declaring one
    const wanted = new Map<AllowanceResource, Derived>();
    for (const reference of features.values()) {
        const resource = (await catalog(reference.packageId)).find(reference)?.definition.allowance;
        const resolved = resource === undefined ? null : Entitlement.resolve(granted, reference);
        if (resource !== undefined && resolved !== null) {
            wanted.set(resource, { resource, grant: allowed(resolved) });
        }
    }

    return wanted;
}

/** Derive what a resolved grant allows. */
function allowed(resolved: Resolution): AllowanceGrant {
    // keep a metered grant's limit, usage and state
    if (resolved.kind === "metered") {
        return {
            kind: "metered",
            limit: resolved.limit,
            used: resolved.usage,
            state: resolved.state,
        };
    }
    // count a numeric static grant, unlimited when null, or list its values
    else if (resolved.kind === "static") {
        return resolved.value === null || typeof resolved.value === "number"
            ? { kind: "counted", limit: resolved.value }
            : { kind: "listed", values: schema.array(schema.string()).parse(resolved.value) };
    }
    // allow access alone
    else {
        return { kind: "access" };
    }
}

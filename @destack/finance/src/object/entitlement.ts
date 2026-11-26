import type { Decision, FeatureReference } from "@destack/object";
import { defineShape, type Shape, type ShapeKey } from "@destack/object";
import { space, Space, type Tenancy } from "@destack/account/object";
import type { DatabaseConnection, Snapshot, Watch } from "@destack/db/database";
import { type Identifier, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import type { Allowance, EntitlementResource } from "../feature/allowance.ts";
import { FeatureLimit } from "../feature/feature.ts";
import { Allotment, type Resolution } from "./allotment.ts";
import { product } from "./product.ts";
import { productFeature } from "./product-feature.ts";
import { quota } from "./quota.ts";
import { seller } from "./seller.ts";
import { Grant } from "./grant.ts";
import { subscription, subscriptionItem } from "./subscription.ts";

/** A use of an account's plan: a platform resource, a space's tenancy, or a package's feature. */
export type EntitlementUse =
    | { readonly kind: "resource"; readonly resource: Exclude<EntitlementResource, "tenancy"> }
    | { readonly kind: "tenancy"; readonly tenancy: Tenancy }
    | { readonly kind: "feature"; readonly feature: FeatureReference };

/** The rows an account's plan derives from, which its spaces replicate from the universe. */
export const entitlementShape = defineShape({
    name: "entitlement",
    parameters: schema.object({ account: schema.identifier("account") }),
    objects: [seller, product, productFeature, subscription, subscriptionItem, quota],
    queries: ({ account: accountId }) => [
        // replicate every seller's default products with their features, the catalog's few rows
        seller.query({ scopes: "every" }),
        product.query({ scopes: "every", where: { isDefault: true } }),
        productFeature.query({ scopes: "every" }),
        // replicate the account's own subscriptions, their items and its quotas
        subscription.query({ scopes: "every", where: { accountId } }),
        // TODO #Performance: select the items by their subscription's account, not by a read check over every item
        subscriptionItem.query({ scopes: "every", permission: "read" }),
        quota.query({ scopes: "every", where: { accountId } }),
    ],
});

/** Every account's plan rows, which the universe replicates from the sellers' home. */
export const financeShape = defineShape({
    name: "finance",
    parameters: schema.object({}),
    objects: [seller, product, productFeature, subscription, subscriptionItem, quota],
    queries: () => [
        seller.query({ scopes: "every" }),
        product.query({ scopes: "every", where: { isDefault: true } }),
        productFeature.query({ scopes: "every" }),
        subscription.query({ scopes: "every" }),
        subscriptionItem.query({ scopes: "every" }),
        quota.query({ scopes: "every" }),
    ],
});

/** Follow the plan of the account of each space a database serves, from the universe. */
export const entitlementFollow = entitlementShape.per(space, {
    key: async (row, view) =>
        (await Space.isServing(view, row.id))
            ? { key: { scope: Scope.universe.id, account: row.scope } }
            : undefined,
});

/** The condition of a use before the account's plan finishes replicating: not yet known. */
const PENDING: Decision = {
    status: "unknown",
    reason: "Pending",
    message: "waiting for the account's plan",
};

/** An account's plan: what its subscriptions, else each seller's default product, allow it now. */
export class Entitlement {
    /** The account. */
    readonly accountId: Identifier<"account">;
    /** The database replicating the account's plan. */
    readonly #database: DatabaseConnection;
    /** The follow the plan's rows replicate through, pending until it synced. */
    readonly #replica: { readonly shape: Shape; readonly key: ShapeKey<object> };

    /** Read an account's plan from a database replicating it through a follow. */
    constructor(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
        replica: { readonly shape: Shape; readonly key: ShapeKey<object> },
    ) {
        this.accountId = accountId;
        this.#database = database;
        this.#replica = replica;
    }

    /** Decide a use of the plan now, pending until the plan replicated. */
    async decide(use: EntitlementUse, now: number): Promise<Decision> {
        // wait while the plan replicates
        if (!(await this.#replica.shape.isSynced(this.#database, this.#replica.key))) {
            return PENDING;
        }

        // find what the plan allows of the use
        const resolutions = await this.resolve(this.#database.snapshot(), now);
        const resolved = resolutions.find((each) => Entitlement.#matches(each, use));
        const used = Entitlement.#describe(use);

        // refuse a use no plan grants, and a tenancy the plan leaves out
        const listed = schema
            .array(schema.string())
            .safeParse(resolved?.kind === "static" ? resolved.value : undefined);
        const isUnlisted =
            use.kind === "tenancy" && (!listed.success || !listed.data.includes(use.tenancy));
        if (resolved === undefined || isUnlisted) {
            return {
                status: "false",
                reason: "Unentitled",
                message: `no plan of the account grants ${used}`,
            };
        }
        // refuse a use blocked at its limit
        else if (resolved.kind === "metered" && resolved.state === "blocked") {
            const limit =
                resolved.limit === null ? "its spending limit" : `its limit of ${resolved.limit}`;

            return { status: "false", reason: "Blocked", message: `${used} is at ${limit}` };
        }
        // allow any other use
        else {
            return { status: "true", reason: "Within", message: `${used} within the plan` };
        }
    }

    /** Resolve each feature the plan grants now, a metered one at its quota once derived. */
    async resolve(reader: Pick<Snapshot, "read">, now: number): Promise<Resolution[]> {
        // allot the features of each grant, a metered one unused and within until its quota says more
        const allotments: Allotment[] = [];
        for (const grant of await Grant.list(this.accountId, now, reader)) {
            for (const feature of grant.features) {
                const isMetered = feature.kind === "metered";
                allotments.push({
                    packageId: feature.packageId,
                    feature: feature.feature,
                    kind: feature.kind,
                    resource: feature.resource,
                    value: feature.kind === "static" ? feature.value : null,
                    limit:
                        isMetered && feature.value !== null
                            ? FeatureLimit.parse(feature.value)
                            : null,
                    usage: isMetered ? 0 : null,
                    resetAt: null,
                    state: isMetered ? "within" : null,
                    grant: grant.reference,
                    grantedAt: grant.grantedAt,
                    period: grant.period,
                });
            }
        }

        // resolve each feature across its grants, taking a metered one's quota where finance derived it
        const grouped = Map.groupBy(allotments, (allotment) => Entitlement.#key(allotment));
        const rows = await reader
            .read(quota.table)
            .findMany({ where: { accountId: this.accountId } });
        const quotas = new Map(rows.map((row) => [Entitlement.#key(row), row]));

        return [...grouped.values()].map((granted): Resolution => {
            const resolved = Allotment.resolve(granted);
            const row = quotas.get(Entitlement.#key(resolved));

            return resolved.kind === "metered" && row !== undefined
                ? {
                      ...resolved,
                      limit: row.limit,
                      usage: row.usage,
                      resetAt: row.resetAt,
                      state: row.state,
                      periodStart: row.periodStart,
                  }
                : resolved;
        });
    }

    /** Watch the rows a decision reads, keyed to the account of a watching row's `scope`. */
    static readonly watches: readonly Watch[] = [
        { table: subscription.table, fields: ["status", "accountId"], on: { scope: "accountId" } },
        { table: quota.table, fields: ["state", "limit"], on: { scope: "accountId" } },
        {
            table: product.table,
            where: { isDefault: true },
            fields: ["isDefault", "archivedAt"],
            isListing: true,
        },
        { table: productFeature.table, fields: ["value", "kind", "resource"], isListing: true },
    ];

    /** Key a grant or resolution by its feature. */
    static #key(allowance: Pick<Allowance, "packageId" | "feature">): string {
        return `${allowance.packageId}/${allowance.feature}`;
    }

    /** Report whether a resolved feature serves a use: its resource, its tenancy or itself. */
    static #matches(resolved: Resolution, use: EntitlementUse): boolean {
        // match a resource or a tenancy by the resource the feature allows
        if (use.kind === "resource" || use.kind === "tenancy") {
            return resolved.resource === (use.kind === "resource" ? use.resource : "tenancy");
        }
        // match a feature by package and name
        else {
            return (
                resolved.packageId === use.feature.packageId &&
                resolved.feature === use.feature.name
            );
        }
    }

    /** Describe a use as people read it. */
    static #describe(use: EntitlementUse): string {
        // describe a resource by its name
        if (use.kind === "resource") {
            return use.resource;
        }
        // describe a tenancy
        else if (use.kind === "tenancy") {
            return `${use.tenancy} tenancy`;
        }
        // describe a feature
        else {
            return `feature ${use.feature.name}`;
        }
    }
}

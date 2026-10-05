import { account, organisation } from "@destack/account/object";
import type { WorkloadIdentity } from "@destack/account/client";
import type { DatabaseConnection } from "@destack/db";
import { ObjectWatch } from "@destack/object";
import { ObjectServer, Subscriber } from "@destack/object/server";
import type { JsonValue } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { CallKey } from "@destack/service/request";
import type { ServiceImplementation } from "@destack/service/server";
import { FeatureReference } from "../feature/feature.ts";
import { FeatureCatalog } from "../feature/index.ts";
import {
    customer,
    Customer,
    entitlement,
    type FeatureGrant,
    invoice,
    meterEvent,
    price,
    product,
    productFeature,
    type PriceTerms,
    purchase,
    seller,
    Seller,
    subscription,
    subscriptionItem,
} from "../object/index.ts";
import { financeService } from "../service/index.ts";
import { entitle, measure, type OpenRelease } from "./entitlement.ts";

/** The database, workload identity and releases the finance service serves with. */
export interface FinanceOptions {
    /** The finance database, with copies of its residency's accounts and organisations. */
    readonly database: DatabaseConnection;
    /** The service's placement in its residency, which follows the account service's copies. */
    readonly identity: WorkloadIdentity;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** Open the build declaring a package's features and meters. */
    readonly release: OpenRelease;
    /** Report committed external work that fails to settle. */
    readonly report?: (error: unknown) => void;
}

/** The finance service with the object server executing its methods. */
export interface FinanceImplementation extends ServiceImplementation {
    /** The object server executing the finance service's methods. */
    readonly objects: ObjectServer<ReturnType<typeof servedObjects>>;
}

/** Implement the finance service over a residency's database, following the account service and deriving entitlements under a controller. */
export function implementFinance(options: FinanceOptions): FinanceImplementation {
    // serve the finance objects, following the account service's copies as the placement
    const { identity } = options;
    const objects: ObjectServer<ReturnType<typeof servedObjects>> = new ObjectServer({
        objects: servedObjects(options.release),
        policies: [account, organisation],
        database: options.database,
        callKey: options.callKey,
        origin: { package: financeService.package, service: financeService.name },
        subscriber: Subscriber.of(identity.publisher(), () =>
            objects.source.workloadSubscriptions(identity.placementId),
        ),
        ...(options.report === undefined ? {} : { report: options.report }),
    });

    return { ...objects.implement(financeService), objects };
}

/** Serve the finance objects, checking grants against their features' declarations and deriving entitlements. */
export function servedObjects(release: OpenRelease) {
    return {
        customer: servedCustomer(release),
        seller,
        product,
        productFeature: productFeature.handle({
            create: {
                authorize: (call) => requireGrant(call.input, call.scope, call.database, release),
            },
            update: {
                authorize: (call) =>
                    requireGrant(
                        { ...call.requireTarget(), ...call.input },
                        call.scope,
                        call.database,
                        release,
                    ),
            },
        }),
        price,
        subscription: subscription.handle({
            create: async (call, next) => {
                await Customer.require(call.database, account.identifier(call.scope));

                return next();
            },
            cancel: (call) => call.update({ status: "canceled", canceledAt: call.now }),
        }),
        subscriptionItem: subscriptionItem.handle({
            create: (call, next) => {
                requireType(call.input.terms, "recurring", "a subscription item");

                return next();
            },
        }),
        purchase: purchase.handle({
            create: async (call, next) => {
                // require a customer buying a one-time price without metered grants
                await Customer.require(call.database, account.identifier(call.scope));
                requireType(call.input.terms, "one_time", "a purchase");
                await requireUnmetered(call.input.grants, release);

                return next();
            },
            pay: (call) => call.update({ status: "paid", paidAt: call.now }),
        }),
        entitlement,
        meterEvent: meterEvent.handle({
            create: async (call, next) => {
                // keep the event and count it into its meter's entitlements
                const event = await next();
                await measure(call, event, release);

                return event;
            },
        }),
        invoice,
    };
}

/** Serve customers, deriving each account's entitlements again after its subscriptions and purchases change. */
function servedCustomer(release: OpenRelease) {
    return customer
        .handle({
            entitle: async (call) => {
                await entitle(call, release);

                return call.requireTarget();
            },
        })
        .control({
            // NOTE #Performance: derive every account's entitlements again at each start
            pending: {},
            key: (row) => ({ scope: row.scope }),
            watches: [
                ObjectWatch.of(subscription.table, (row) => [{ scope: row.scope }]),
                ObjectWatch.of(subscriptionItem.table, (row) => [{ scope: row.scope }]),
                ObjectWatch.of(purchase.table, (row) => [{ scope: row.scope }]),
            ],
            reconcile: async (reconciliation) => {
                await reconciliation.execute("entitle", reconciliation.rows);

                return undefined;
            },
        });
}

/** Require a seller's grant of a feature: one of a package it publishes, with a value or limit its declaration accepts. */
async function requireGrant(
    grant: { readonly packageId: string; readonly feature: string; readonly value?: JsonValue },
    scope: string,
    database: DatabaseConnection,
    release: OpenRelease,
): Promise<void> {
    // refuse a feature of a package another account publishes
    const reference = FeatureReference.parse({ packageId: grant.packageId, name: grant.feature });
    const reader = await release(reference.packageId);
    await Seller.requirePublisher(database, account.identifier(scope), reader.manifest.package);

    // refuse a value the feature's declaration rejects
    const catalog = await FeatureCatalog.read(reader);
    catalog.feature(reference).requireGrant(grant.value ?? null);
}

/** Require the price terms a subscription item or purchase keeps to bill each period or once. */
function requireType(terms: PriceTerms, type: PriceTerms["type"], buyer: string): void {
    // refuse a price of the other type
    if (terms.type !== type) {
        throw new ServiceError("BAD_REQUEST", {
            message: `${buyer} needs a ${type === "recurring" ? "recurring" : "one-time"} price`,
        });
    }
}

// TODO #Incomplete: grant one-time metered usage through credit grants
/** Refuse a purchase granting a metered feature, whose one-time usage credit grants cover. */
async function requireUnmetered(
    grants: readonly FeatureGrant[],
    release: OpenRelease,
): Promise<void> {
    for (const grant of grants) {
        // read the granted feature's kind
        const reference = { packageId: grant.packageId, name: grant.feature };
        const catalog = await FeatureCatalog.read(await release(grant.packageId));

        // refuse a metered one
        if (catalog.feature(reference).definition.kind === "metered") {
            throw new ServiceError("BAD_REQUEST", {
                message: `a purchase cannot grant metered feature ${FeatureReference.key(reference)}`,
            });
        }
    }
}

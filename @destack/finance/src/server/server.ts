import type { WorkloadIdentity } from "@destack/account/client";
import {
    account,
    allowance,
    allowanceShape,
    key,
    machine,
    organisation,
    universeAllowanceShape,
} from "@destack/account/object";
import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
import type { Event, EventKind, EventStore } from "@destack/event";
import type { Registry } from "@destack/forge/client";
import { serveActivities } from "@destack/notification/server";
import type { CallOf } from "@destack/object";
import { ObjectServer, Subscriber } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import { type JsonValue, present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { CallKey } from "@destack/service/request";
import { type ServiceContext, type ServiceImplementation } from "@destack/service/server";
import { Scope } from "@destack/sync";
import { Catalog } from "../catalog/catalog.ts";
import { usage } from "../meter/usage.ts";
import {
    charge,
    checkoutSession,
    customer,
    Customer,
    entitlement,
    invoice,
    price,
    Price,
    type PriceTerms,
    product,
    productFeature,
    seller,
    Seller,
    subscription,
    subscriptionItem,
    usageLimit,
} from "../object/index.ts";
import { type PaymentProvider, ProviderEvent } from "../provider/provider.ts";
import { Rate } from "../rate/rate.ts";
import { financeService } from "../service/index.ts";
import { EntitlementController, notifyState } from "./entitlement.ts";
import { Billing } from "./payment.ts";
import { syncSubscription } from "./subscription.ts";
import { MeterUsage, UsageController } from "./usage.ts";
import { CatalogReference } from "../catalog/reference.ts";

/** The database, workload identity and registry the finance service serves with. */
export interface FinanceOptions {
    /** The finance database, with copies of its residency's accounts and organisations. */
    readonly database: DatabaseConnection;
    /** The service's placement in its residency, which follows the account service's copies. */
    readonly identity: WorkloadIdentity;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The registry opening the releases of packages declaring features, meters and SKUs. */
    readonly registry: Pick<Registry, "open">;
    /** The distribution tag naming each package's current release, such as latest, or dev in a dev universe. */
    readonly tag: string;
    /** The payment provider charging buyers, absent in a universe whose accounts run on default products alone. */
    readonly provider?: PaymentProvider;
    /** The usage events routed to the residency's paying accounts, kept in the finance database and its bucket. */
    readonly events: EventStore;
    /** Report committed external work that fails to settle. */
    readonly report?: (error: unknown) => void;
}

/** The finance service with the object server executing its methods. */
export interface FinanceImplementation extends ServiceImplementation {
    /** The object server executing the finance service's methods. */
    readonly objects: ObjectServer<ReturnType<typeof serveFinance>>;
}

/** The announcements of the accounts' usage limits. */
const activities = serveActivities({ notifications: [usageLimit] });

/** Implement the finance service over a residency's database. */
export function implementFinance(options: FinanceOptions): FinanceImplementation {
    // serve the finance objects, following the account service's copies as the placement
    const { identity } = options;
    const meterUsage = new MeterUsage(options.events);
    const catalog = Catalog.tagged(options.registry, options.tag);
    const objects: ObjectServer<ReturnType<typeof serveFinance>> = new ObjectServer({
        objects: serveFinance(catalog, options.provider),
        policies: [account, organisation, machine, key],
        database: options.database,
        callKey: options.callKey,
        origin: { package: financeService.package, service: financeService.name },
        subscriber: Subscriber.of(identity.publisher(), () =>
            objects.source.workloadSubscriptions(identity.placementId),
        ),
        shapes: [allowanceShape, universeAllowanceShape],
        ...(options.report === undefined ? {} : { report: options.report }),
    });

    // take the provider's webhook deliveries at their path, verified by their signatures
    const { provider } = options;
    const path = provider === undefined ? undefined : `/finance/webhooks/${provider.name}`;
    const route = async (request: Request): Promise<Response | undefined> => {
        if (provider === undefined || new URL(request.url).pathname !== path) {
            return undefined;
        } else if (request.method !== "POST") {
            return new Response(null, { status: 405 });
        }
        await ProviderEvent.receive(objects, provider, request);

        return new Response(null, { status: 204 });
    };

    // rate the usage routed here, derive each account's entitlements, and keep the events' segments
    const report = options.report ?? (() => undefined);
    const controllers = [
        new UsageController(objects, meterUsage, catalog, report),
        new EntitlementController(objects, catalog, meterUsage),
        ...options.events.controllers,
    ];

    return { ...objects.implement(financeService, controllers), objects, route };
}

/** Admit usage of accounts the caller serves. */
export function admitUsage(
    objects: ObjectServer,
): (context: ServiceContext, kind: EventKind, events: readonly Event[]) => Promise<void> {
    return async (context, kind, events) => {
        // take usage alone, the one kind routed here
        if (kind.key !== usage.key) {
            throw new ServiceError("FORBIDDEN", {
                message: `finance takes no ${kind.name} events`,
            });
        }

        // require the caller to serve each paying account
        const authorization = context.requireAuthorization();
        for (const scope of new Set(events.map((event) => event.scope))) {
            const [link] = await Scope.chain(Snapshot.live(objects.database), scope);
            const decision =
                link === undefined
                    ? undefined
                    : await authorization.check(account.permission("serve"), link.object);
            if (decision?.isAllowed !== true) {
                throw new ServiceError("NOT_FOUND", { message: `no account ${scope}` });
            }
        }
    };
}

/** Serve the finance objects. */
export function serveFinance(
    catalog: (packageId: PackageId) => Promise<Catalog>,
    provider?: PaymentProvider,
) {
    const billing = new Billing(provider);

    return {
        customer,
        seller: serveSellers(billing),
        product: product.handle({
            update: {
                authorize: (call) =>
                    requireDefault({ ...call.requireTarget(), ...call.input }, call.database),
            },
        }),
        productFeature: serveProductFeatures(catalog),
        price: price.handle({
            create: {
                authorize: (call) => requirePrice(call.input, call.database, catalog),
                prepare: (call) => billing.preparePrice(call),
                handler: (call, next) => billing.createPrice(call, next),
            },
        }),
        subscription: serveSubscriptions(billing),
        subscriptionItem,
        checkoutSession: serveCheckouts(billing),
        entitlement: serveEntitlements(),
        allowance,
        announcement: activities.announcement,
        charge,
        invoice,
    };
}

/** Serve entitlements, notifying usage state changes. */
function serveEntitlements() {
    return entitlement.handle({
        create: async (call, next) => {
            // notify the first state
            const row = await next();
            await notifyState(call, row, null);

            return row;
        },
        update: async (call, next) => {
            // notify a changed state
            const previous = call.requireTarget().state;
            const row = await next();
            await notifyState(call, row, previous);

            return row;
        },
    });
}

/** Serve product features, granting only declared features of packages the seller's account publishes. */
function serveProductFeatures(catalog: (packageId: PackageId) => Promise<Catalog>) {
    return productFeature.handle({
        create: {
            authorize: (call) => requireGrant(call.input, call.scope, call.database, catalog),
        },
        update: {
            authorize: (call) =>
                requireGrant(
                    { ...call.requireTarget(), ...call.input },
                    call.scope,
                    call.database,
                    catalog,
                ),
        },
    });
}

/** Serve sellers, opening each one's connected account at the provider. */
function serveSellers(billing: Billing) {
    return seller.handle({
        create: {
            prepare: async (call) => ({
                providerId:
                    billing.provider === undefined
                        ? null
                        : await billing.provider.openSeller(
                              call.input.country,
                              present(call.idempotencyKey, "a seller's idempotency key"),
                          ),
            }),
            handler: (call, next) =>
                next(call.with({ input: { ...call.input, providerId: call.prepared.providerId } })),
        },
        onboard: {
            prepare: async (call) => {
                const providerId = call.requireTarget().providerId;
                if (providerId === null) {
                    throw new ServiceError("PRECONDITION_FAILED", {
                        message: "the seller has no connected account to onboard",
                    });
                }

                return { url: await billing.requireProvider().onboard(providerId, call.input) };
            },
            handler: async (call) => call.prepared,
        },
    });
}

/** Serve checkout sessions, settled by the provider's webhook. */
function serveCheckouts(billing: Billing) {
    return checkoutSession.handle({
        create: {
            authorize: (call) => Customer.require(call.database, account.identifier(call.scope)),
            prepare: (call) => billing.prepareCheckout(call),
            handler: (call, next) => billing.openCheckout(call, next),
        },
        settle: (call) => Billing.settle(call),
    });
}

/** Serve subscriptions: followed as the provider bills them, changed between products, and closed each period onto its invoice. */
function serveSubscriptions(billing: Billing) {
    return subscription.handle({
        create: async (call, next) => {
            await Customer.require(call.database, account.identifier(call.scope));

            return next();
        },
        closePeriod: {
            prepare: (call) => billing.prepareClose(call),
            handler: (call) => Billing.close(call),
        },
        cancel: (call) => call.update({ status: "canceled", canceledAt: call.now }),
        change: {
            prepare: (call) => billing.prepareChange(call),
            handler: (call) => billing.change(call),
        },
        sync: (call) => syncSubscription(call),
    });
}

/** Require a seller's grant of a feature it publishes. */
async function requireGrant(
    grant: { readonly packageId: string; readonly feature: string; readonly value?: JsonValue },
    scope: string,
    database: DatabaseConnection,
    catalog: (packageId: PackageId) => Promise<Catalog>,
): Promise<void> {
    // refuse a feature of a package another account publishes
    const reference = CatalogReference.parse({ packageId: grant.packageId, name: grant.feature });
    const declared = await catalog(reference.packageId);
    await Seller.requirePublisher(database, account.identifier(scope), declared.package);

    // refuse a value the feature's declaration rejects
    declared.feature(reference).requireGrant(grant.value ?? null);
}

/** Require a default product without prices, at most one per seller. */
async function requireDefault(
    row: { readonly id: string; readonly isDefault?: boolean },
    database: DatabaseConnection,
): Promise<void> {
    // refuse marking a product with prices as the default
    if (row.isDefault !== true) {
        return;
    }
    const [priced] = await database
        .select({ id: price.table.id })
        .from(price.table)
        .where(eq(price.table.parentId, product.identifier(row.id)))
        .limit(1);
    if (priced !== undefined) {
        throw new ServiceError("BAD_REQUEST", {
            message: `product ${row.id} has prices, so it cannot be every account's default`,
        });
    }
}

/** Require a price of a product other than a default one, billing a declared meter, and its fee to cover the usage it includes. */
async function requirePrice(
    input: CallOf<typeof price, "create">["input"],
    database: DatabaseConnection,
    catalog: (packageId: PackageId) => Promise<Catalog>,
): Promise<void> {
    // refuse a price of a default product
    const [owner] = await database
        .select({ isDefault: product.table.isDefault })
        .from(product.table)
        .where(eq(product.table.id, product.identifier(String(input.parentId))));
    if (owner?.isDefault === true) {
        throw new ServiceError("BAD_REQUEST", {
            message: `product ${String(input.parentId)} is every account's default, which has no prices`,
        });
    }

    // refuse a meter no package declares
    const { recurring } = input;
    if (recurring.usage === "metered") {
        (await catalog(recurring.meter.packageId)).meter(recurring.meter);
    }

    // refuse a fee below the usage it includes
    requireFee(Price.terms(input));
}

/** Refuse a fee each period below the usage it includes, which would bill the buyer for usage they never paid. */
function requireFee(terms: PriceTerms): void {
    // read the fee of one unit of a licensed price
    if (terms.recurring.usage !== "licensed") {
        return;
    }
    const fee = Number(Rate.price(terms, 1));

    // refuse a fee below the included usage
    if (fee < terms.includedUsage) {
        throw new ServiceError("BAD_REQUEST", {
            message: `a price's included usage of ${terms.includedUsage} exceeds its fee of ${fee} ${terms.currency}`,
        });
    }
}

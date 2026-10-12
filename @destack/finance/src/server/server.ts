import {
    account,
    allowance,
    allowanceShape,
    key,
    machine,
    organisation,
    universeAllowanceShape,
} from "@destack/account/object";
import type { accountService } from "@destack/account/service";
import { TABLE } from "@destack/db";
import type { EventStore } from "@destack/event";
import type { Registry } from "@destack/forge/client";
import { serveActivities } from "@destack/notification/server";
import { type ObjectExtension, Stack } from "@destack/object/server";
import { Digest, type Identifier, type JsonObject, JsonValue } from "@destack/schema";
import type { ServiceClient } from "@destack/service/client";
import { ServiceError } from "@destack/service/error";
import { WEBHOOK_PATH } from "@destack/service/workload";
import { Catalog } from "../catalog/catalog.ts";
import { FeatureGrant } from "../feature/grant.ts";
import {
    charge,
    checkoutSession,
    customer,
    Customer,
    entitlement,
    Entitlement,
    invoice,
    price,
    Price,
    product,
    productFeature,
    ProductFeature,
    seller,
    subscription,
    Subscription,
    subscriptionItem,
    usageLimit,
} from "../object/index.ts";
import { type PaymentProvider, ProviderEvent } from "../provider/provider.ts";
import { financeService } from "../service/index.ts";
import { EntitlementController } from "./entitlement.ts";
import { Billing } from "./payment.ts";
import { Metering } from "../meter/metering.ts";
import { UsageController } from "./usage.ts";

/** The hex digits of a price's terms digest its declaration name carries: 48 bits. */
const TERMS_DIGEST_LENGTH = 12;

/** The account directory, registry, provider and events the finance extension serves with. */
export interface FinanceOptions {
    /** The universe's account directory, which finds the seller of each handle. */
    readonly accounts: Pick<ServiceClient<typeof accountService>, "directory">;
    /** The residency home finance serves, where its rows live. */
    readonly home: Identifier<"space">;
    /** The registry opening the releases of packages declaring features, meters and SKUs. */
    readonly registry: Pick<Registry, "open">;
    /** The distribution tag of each package's current release, such as latest or dev. */
    readonly tag: string;
    /** The payment provider charging buyers, absent if accounts run on default products alone. */
    readonly provider: PaymentProvider | undefined;
    /** The space's events, with the usage routed to the residency's paying accounts. */
    readonly events: EventStore;
    /** Report committed external work that fails to settle, and uses their declarations refuse. */
    readonly report: (error: unknown) => void;
}

/** The announcements of the accounts' usage limits. */
const activities = serveActivities({ notifications: [usageLimit] });

/** Extend a residency home's store with finance: its objects, webhooks and controllers. */
export function implementFinance(options: FinanceOptions): ObjectExtension {
    // find each handle's seller to the home's buyers through the account directory
    const { home, provider } = options;
    const catalog = Catalog.tagged(options.registry, options.tag);
    const publisher = (handle: string) => options.accounts.directory.publisher({ handle, home });
    const metering = new Metering(options.events, catalog, publisher, options.report);
    const path = provider === undefined ? undefined : `${WEBHOOK_PATH}/${provider.name}`;

    return {
        service: financeService,
        objects: serveFinance(metering, provider, home),
        policies: [account, organisation, machine, key],
        shapes: [allowanceShape, universeAllowanceShape],
        serve: (store) => ({
            // take the provider's webhook deliveries at their path below the machine's webhook mount
            fetch: async (request: Request): Promise<Response | undefined> => {
                if (provider === undefined || new URL(request.url).pathname !== path) {
                    return undefined;
                } else if (request.method !== "POST") {
                    return new Response(null, { status: 405 });
                }
                await ProviderEvent.receive(store, home, provider, request);

                return new Response(null, { status: 204 });
            },

            // rate the usage routed here and derive each account's entitlements
            controllers: [new UsageController(store, home), new EntitlementController(store, home)],
        }),
    };
}

/** Serve the finance objects. */
export function serveFinance(
    metering: Metering,
    provider: PaymentProvider | undefined,
    home: string,
) {
    const { catalog } = metering;
    const billing = new Billing(provider, home);

    return {
        customer,
        seller: serveSellers(billing),
        product: product.implement({
            methods: {
                update: {
                    authorize: (call) =>
                        Price.requireNone({ ...call.requireTarget(), ...call.input }, call),
                },
            },
            declaration: {
                // apply after the seller its prices check against
                after: [seller],
                // offer a declared product again, unless it says otherwise
                resolve: async (_name, declared) => ({
                    active: true,
                    ...JsonValue.object(declared),
                }),
                // archive a product its stack stops declaring, which subscriptions keep referencing
                retire: "archived",
            },
        }),
        productFeature: serveProductFeatures(metering),
        price: price.implement({
            methods: {
                create: {
                    authorize: (call) =>
                        Price.require(
                            call.input.parentId,
                            Price.terms(call.input),
                            call.view(),
                            catalog,
                        ),
                },
            },
            declaration: {
                after: [product],
                // name each price by its terms, so new terms make a new price and archive the old
                collect: async (document) => {
                    const named = Stack.nested(price, product, document).map(
                        async ([name, value]): Promise<[string, JsonObject]> => {
                            // digest the declared price's terms
                            const declared = JsonValue.object(value);
                            const terms = Price.terms(price.table[TABLE].decode(declared));
                            const digest = await Digest.json(terms);

                            return [`${name}-${digest.slice(0, TERMS_DIGEST_LENGTH)}`, declared];
                        },
                    );

                    return Object.fromEntries(await Promise.all(named));
                },
                // refuse a price the API refuses, and offer a declared price again unless it says otherwise
                resolve: async (name, declared, stack) => {
                    // check the terms under the declared product
                    const parentId = await stack.require(product, Stack.parent(name));
                    const fields = JsonValue.object(declared);
                    const terms = Price.terms(price.table[TABLE].decode(fields));
                    await Price.require(parentId, terms, stack.transaction.view(), catalog);

                    return { active: true, ...fields };
                },
                retire: "archived",
            },
        }),
        subscription: serveSubscriptions(billing),
        subscriptionItem,
        checkoutSession: serveCheckouts(billing),
        entitlement: serveEntitlements(metering),
        allowance,
        announcement: activities.announcement,
        charge: charge.implement({
            methods: {
                rate: (call) => metering.rate(call, call.input.accountId, call.input.after),
            },
        }),
        invoice,
    };
}

/** Serve entitlements, deriving each account's and notifying usage state changes. */
function serveEntitlements(metering: Metering) {
    return entitlement.implement({
        methods: {
            derive: async (call) => {
                // derive the account's entitlements and allowances
                await Entitlement.derive(call, call.input.accountId, metering);

                return {};
            },
            create: async (call, next) => {
                // notify the first state
                const row = await next();
                await Entitlement.notify(call, row, null);

                return row;
            },
            update: async (call, next) => {
                // notify a changed state
                const previous = call.requireTarget().state;
                const row = await next();
                await Entitlement.notify(call, row, previous);

                return row;
            },
        },
    });
}

/** Serve product features, granting only declared features of the seller's own packages. */
function serveProductFeatures(metering: Metering) {
    const { catalog, publisher } = metering;
    return productFeature.implement({
        declaration: {
            after: [product],
            // refuse a grant the API refuses
            resolve: async (name, declared, stack) => {
                // check the grant under the declared product
                const parentId = await stack.require(product, Stack.parent(name));
                const grant = { ...FeatureGrant.parse(declared), parentId };
                await ProductFeature.requireGrant(
                    grant,
                    stack.transaction.view(),
                    catalog,
                    publisher,
                );

                return declared;
            },
        },
        methods: {
            create: {
                authorize: (call) =>
                    ProductFeature.requireGrant(call.input, call.view(), catalog, publisher),
            },
            update: {
                authorize: (call) =>
                    ProductFeature.requireGrant(
                        { ...call.requireTarget(), ...call.input },
                        call.view(),
                        catalog,
                        publisher,
                    ),
            },
        },
    });
}

/** Serve sellers: the platform's own selling at once, others through a connected account. */
function serveSellers(billing: Billing) {
    return seller.implement({
        declaration: {
            // declare the platform's own seller active, which sells on the provider account itself
            resolve: async (_name, declared, stack) => {
                const accountId = account.identifier(await stack.enclosing());
                const status = billing.isPlatform(accountId) ? { status: "active" } : {};

                return { ...JsonValue.object(declared), ...status };
            },
            // keep a seller its stack stops declaring, which subscriptions and charges reference
            retire: "kept",
        },
        methods: {
            create: async (call, next) => {
                // activate the platform's own seller, which sells on the provider account itself
                const created = await next();
                if (!billing.isPlatform(created.accountId)) {
                    return created;
                }
                const [activated] = await call.mutate(call.service, [
                    seller.call("activate", { id: created.id }),
                ]);

                return activated;
            },
            onboard: {
                prepare: async (call) => {
                    // refuse the platform's own seller, which has no connected account
                    const row = call.requireTarget();
                    if (billing.isPlatform(row.accountId)) {
                        throw new ServiceError("PRECONDITION_FAILED", {
                            message: "the platform sells on the provider account itself",
                        });
                    }

                    // open the connected account once, then its onboarding page
                    const provider = billing.requireProvider();
                    const providerId =
                        row.providerId ??
                        (await provider.openSeller(row.country, `seller/${row.id}`));
                    const url = await provider.onboard(providerId, call.input);

                    return { url, providerId };
                },
                handle: async (call) => {
                    // record a connected account opened now
                    const { url, providerId } = call.prepared;
                    const row = call.requireTarget();
                    if (row.providerId === null) {
                        await call.mutate(call.service, [
                            seller.call("record", { id: row.id, providerId }),
                        ]);
                    }

                    return { url };
                },
            },
        },
    });
}

/** Serve checkout sessions, settled by the provider's webhook. */
function serveCheckouts(billing: Billing) {
    return checkoutSession.implement({
        methods: {
            create: {
                authorize: (call) => Customer.require(call, call.input.accountId),
                prepare: (call) => billing.prepareCheckout(call),
                handle: (call, next) => billing.openCheckout(call, next),
                settle: (call, settled) => billing.settleCheckout(call, settled),
            },
            settle: (call) => Billing.settle(call),
        },
    });
}

/** Serve subscriptions: follow provider billing, change products, close periods onto invoices. */
function serveSubscriptions(billing: Billing) {
    return subscription.implement({
        methods: {
            create: async (call, next) => {
                await Customer.require(call, call.input.accountId);

                return next();
            },
            closePeriod: {
                prepare: (call) => billing.prepareClose(call),
                handle: (call) => Billing.close(call),
            },
            change: {
                prepare: (call) => billing.prepareChange(call),
                handle: (call) => billing.change(call),
            },
            sync: (call) => Subscription.sync(call),
        },
    });
}

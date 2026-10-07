import { account } from "@destack/account/object";
import { and, eq, type DatabaseConnection } from "@destack/db";
import type { Call, CallOf, NextOf, PreparedCallOf } from "@destack/object";
import { present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Order, PaymentProvider } from "../provider/provider.ts";
import {
    Charge,
    checkoutSession,
    type CheckoutSession,
    customer,
    price,
    Price,
    product,
    type Product,
    Purchasable,
    type ProviderWork,
    seller,
    type Seller,
    type PeriodClose,
    subscription,
    type Subscription,
} from "../object/index.ts";
import { readItemPrices, replaceMetered, startSubscription } from "./subscription.ts";

/** The provider's identifiers of a mirrored price and its product, null without them. */
type PriceMirror = { readonly price: string | null; readonly product: string | null };

/** A product with its recurring prices in one currency. */
export interface Offering {
    /** The product. */
    readonly product: Product;
    /** The seller of the product's account. */
    readonly seller: Seller;
    /** The product's active recurring prices in the currency, empty for a default product. */
    readonly prices: readonly Price[];
}

/** The offerings buyers subscribe to. */
export const Offering = {
    /** Read a product's offering in a currency, a default product's without prices. */
    async read(database: DatabaseConnection, wanted: Purchasable): Promise<Offering> {
        // read the active product and its seller
        const [row] = await database
            .select()
            .from(product.table)
            .where(eq(product.table.id, product.identifier(wanted.product.id)));
        const [sold] = await database
            .select()
            .from(seller.table)
            .where(eq(seller.table.scope, account.identifier(wanted.product.scope)));
        if (row === undefined || !row.active || row.scope !== wanted.product.scope) {
            throw new ServiceError("NOT_FOUND", {
                message: `no product ${wanted.product.id} on sale`,
            });
        } else if (sold === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `account ${row.scope} sells nothing`,
            });
        } else if (row.isDefault) {
            return { product: row, seller: sold, prices: [] };
        }

        // read its active recurring prices in the currency, requiring a licensed fee, 0 for pay-as-you-go
        const prices = await database
            .select()
            .from(price.table)
            .where(
                and(
                    eq(price.table.parentId, row.id),
                    eq(price.table.currency, wanted.currency),
                    eq(price.table.active, true),
                ),
            );
        if (prices.every(isMetered)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `product ${row.id} has no licensed price in ${wanted.currency}`,
            });
        }

        return { product: row, seller: sold, prices };
    },
};

/** Prices, checkouts and plan changes through the payment provider, refused without one. */
export class Billing {
    /** The payment provider, absent in a universe selling nothing. */
    readonly provider: PaymentProvider | undefined;

    /** Bill through a provider, or through none. */
    constructor(provider: PaymentProvider | undefined) {
        this.provider = provider;
    }

    /** Require the provider a sale needs. */
    requireProvider(): PaymentProvider {
        if (this.provider === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: "this universe takes no payments, so it sells nothing",
            });
        }

        return this.provider;
    }

    /** Mirror a new licensed price at the provider, with its product once, leaving a metered price finance rates itself. */
    async preparePrice(call: CallOf<typeof price, "create">): Promise<PriceMirror> {
        // mirror nothing without a provider, nor a metered price
        const terms = Price.terms(call.input);
        if (this.provider === undefined || terms.recurring?.usage === "metered") {
            return { price: null, product: null };
        }

        // create the price and its product at the provider
        const [row] = await call.database
            .select()
            .from(product.table)
            .where(eq(product.table.id, product.identifier(String(call.input.parentId))));

        return this.provider.createPrice(
            present(row, `product ${String(call.input.parentId)}`),
            terms,
            present(call.idempotencyKey, "a price's idempotency key"),
        );
    }

    /** Keep a new price with the provider's identifier, recording its product's on the first one. */
    async createPrice(
        call: PreparedCallOf<typeof price, "create">,
        next: NextOf<typeof price, "create">,
    ): Promise<Price> {
        // keep the price
        const mirrored = call.prepared;
        const created = await next(
            call.with({ input: { ...call.input, providerId: mirrored.price } }),
        );

        // record the product's identifier once
        const [owner] = await call.database
            .select({ providerId: product.table.providerId })
            .from(product.table)
            .where(eq(product.table.id, created.parentId));
        if (mirrored.product !== null && owner?.providerId === null) {
            await call
                .invoke(product)
                .record({ id: created.parentId, providerId: mirrored.product });
        }

        return created;
    }

    /** Open the provider's checkout session for the mirrored customer. */
    async prepareCheckout(call: CallOf<typeof checkoutSession, "create">): Promise<ProviderWork> {
        // mirror the buyer, then open the session for the offering's licensed prices
        const offering = await Offering.read(call.database, purchased(call.input));
        const key = present(call.idempotencyKey, "a checkout's idempotency key");
        const customerId = await this.#mirror(call.database, call.scope, key);
        const session = await this.requireProvider().checkout(
            Billing.#order(offering, customerId),
            `${key}/checkout`,
        );

        return { customer: customerId, session, subscription: null, closed: null };
    }

    /** Keep a checkout session at the provider's page. */
    async openCheckout(
        call: PreparedCallOf<typeof checkoutSession, "create">,
        next: NextOf<typeof checkoutSession, "create">,
    ): Promise<CheckoutSession> {
        // record the buyer's customer, and keep the session with its seller and page
        const offering = await Offering.read(call.database, purchased(call.input));
        const work = call.prepared;
        const session = present(work.session, "the provider's checkout session");
        await Billing.#record(call, work);

        return next(
            call.with({
                input: {
                    ...call.input,
                    seller: { scope: offering.seller.scope, id: offering.seller.id },
                    url: session.url,
                    providerId: session.providerId,
                },
            }),
        );
    }

    /** Change a subscription's prices at the provider, or close its period onto a final invoice and cancel it for the default product. */
    async prepareChange(call: CallOf<typeof subscription, "change">): Promise<ProviderWork> {
        // close the current period onto the final invoice and cancel for the default product
        const offering = await Offering.read(call.database, purchased(call.input));
        const current = call.requireTarget();
        const key = present(call.idempotencyKey, "a change's idempotency key");
        const provider = this.requireProvider();
        const customerId = await this.#mirror(call.database, call.scope, key);
        if (offering.product.isDefault) {
            const period = { start: current.currentPeriodStart, end: current.currentPeriodEnd };
            const invoice = { customer: customerId, providerId: null };
            const closed = await Charge.bill(
                call.database,
                current,
                period,
                invoice,
                provider,
                key,
            );
            await provider.cancel(current.providerId, `${key}/cancel`);

            return { customer: customerId, session: null, subscription: null, closed };
        }

        // change the prices of the subscription
        const order = Billing.#order(offering, customerId);
        const changed = await provider.change(current.providerId, order.prices, `${key}/change`);

        return { customer: customerId, session: null, subscription: changed, closed: null };
    }

    /** Change a subscription's plan as prepared: follow its new prices, or close its period for the default product. */
    async change(call: PreparedCallOf<typeof subscription, "change">): Promise<Subscription> {
        // close the period for the default product, the provider's webhook canceling the subscription
        const work = call.prepared;
        const current = call.requireTarget();
        await Billing.#record(call, work);
        if (work.closed !== null) {
            const period = { start: current.currentPeriodStart, end: current.currentPeriodEnd };
            await Charge.close(call, current, period, work.closed);

            return current;
        }

        // follow the provider's change, with the metered prices and the usage the new prices include
        const offering = await Offering.read(call.database, purchased(call.input));
        const changed = present(work.subscription, "the changed subscription");
        await call.invoke(subscription).sync({ id: current.id, ...changed });
        await replaceMetered(call, current.id, offering.prices.filter(isMetered));
        const includedUsage = offering.prices.reduce(
            (total, each) => total + each.includedUsage,
            0,
        );

        return call.invoke(subscription).record({ id: current.id, includedUsage });
    }

    /** Add an ended period's lines to the provider's draft invoice of its renewal. */
    prepareClose(call: CallOf<typeof subscription, "closePeriod">): Promise<PeriodClose> {
        // bill the period's lines onto the draft under the close's key
        const { periodStart, periodEnd, invoice } = call.input;
        const key = present(call.idempotencyKey, "a close's idempotency key");
        const period = { start: periodStart, end: periodEnd };

        return Charge.bill(
            call.database,
            call.requireTarget(),
            period,
            invoice,
            this.requireProvider(),
            key,
        );
    }

    /** Close an ended period as prepared. */
    static async close(
        call: PreparedCallOf<typeof subscription, "closePeriod">,
    ): Promise<Subscription> {
        const row = call.requireTarget();
        await Charge.close(
            call,
            row,
            { start: call.input.periodStart, end: call.input.periodEnd },
            call.prepared,
        );

        return row;
    }

    /** Settle a session with the subscription the provider started, keeping its metered prices. */
    static async settle(call: CallOf<typeof checkoutSession, "settle">): Promise<CheckoutSession> {
        // start the subscription to the provider's items and the offering's metered prices
        const session = call.requireTarget();
        const provided = call.input.subscription;
        const offering = await Offering.read(call.database, purchased(session));
        const metered = offering.prices
            .filter(isMetered)
            .map((each) => ({ price: each, quantity: 1, providerId: null }));
        const billed = [...(await readItemPrices(call.database, provided.items)), ...metered];
        const started = await startSubscription(call, { seller: session.seller }, billed, provided);

        // complete the session with it
        await call.invoke(checkoutSession).record({ id: session.id, subscriptionId: started.id });

        return call.invoke(checkoutSession).complete({ id: session.id });
    }

    /** Record the provider's identifier of the buying account's customer. */
    static async #record(call: Call, work: ProviderWork): Promise<void> {
        if (work.customer === null) {
            return;
        }
        const [buyer] = await call.database
            .select({ id: customer.table.id })
            .from(customer.table)
            .where(eq(customer.table.scope, account.identifier(call.scope)));
        const id = present(buyer, `the customer of account ${call.scope}`).id;
        await call.invoke(customer).record({ id, providerId: work.customer });
    }

    /** Keep the buying account's billing details at the provider, returning its identifier there. */
    async #mirror(database: DatabaseConnection, scope: string, key: string): Promise<string> {
        const [buyer] = await database
            .select()
            .from(customer.table)
            .where(eq(customer.table.scope, account.identifier(scope)));

        return this.requireProvider().upsertCustomer(
            present(buyer, `the customer of account ${scope}`),
            `${key}/customer`,
        );
    }

    /** Read the order a checkout or change sends: the mirrored prices, a quantity on each but metered ones. */
    static #order(offering: Offering, customerId: string): Order {
        // require the seller's onboarded connected account, which takes the charges
        const sellerId = offering.seller.providerId;
        if (sellerId === null || offering.seller.status !== "active") {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `seller ${offering.seller.id} takes no charges yet`,
            });
        }

        // name each licensed price by the provider's identifier, refusing one never mirrored
        const prices = offering.prices
            .filter((each) => !isMetered(each))
            .map((each) => {
                if (each.providerId === null) {
                    throw new ServiceError("PRECONDITION_FAILED", {
                        message: `price ${each.id} was created without a payment provider`,
                    });
                }

                return { price: each.providerId, quantity: 1 };
            });

        return { customer: customerId, seller: sellerId, prices };
    }
}

/** Read what a checkout or change buys from its input. */
function purchased(input: {
    readonly product: { readonly scope: string; readonly id: string };
    readonly currency: string;
}): Purchasable {
    return Purchasable.parse({ product: input.product, currency: input.currency });
}

/** Decide whether a price bills metered usage, which finance rates and the provider never sees. */
function isMetered(row: Price): boolean {
    return row.recurring?.usage === "metered";
}

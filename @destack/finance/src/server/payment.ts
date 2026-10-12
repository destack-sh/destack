import { Account } from "@destack/account/object";
import type {
    Call,
    CallOf,
    NextOf,
    PreparedCallOf,
    Settled,
    SettlementCall,
} from "@destack/object";
import { type Identifier, present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { CheckoutPages, Order, PaymentProvider } from "../provider/provider.ts";
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
    type PlanChange,
    PriceMirror,
    seller,
    type Seller,
    type PeriodClose,
    subscription,
    Subscription,
    SubscriptionItem,
} from "../object/index.ts";

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
    async read(call: Pick<Call, "read">, wanted: Purchasable): Promise<Offering> {
        // read the active product
        const row = await call.read(product).row({ id: product.identifier(wanted.product.id) });
        if (row === null || !row.active || row.scope !== wanted.product.scope) {
            throw new ServiceError("NOT_FOUND", {
                message: `no product ${wanted.product.id} on sale`,
            });
        }

        // read the seller of the product's account
        const [sold] = await call.read(seller).rows({ accountId: row.accountId });
        if (sold === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `account ${row.accountId} sells nothing`,
            });
        } else if (row.isDefault) {
            return { product: row, seller: sold, prices: [] };
        }

        // read its active recurring prices in the currency, requiring a licensed fee, 0 for pay-as-you-go
        const prices = await call
            .read(price)
            .rows({ parentId: row.id, currency: wanted.currency, active: true });
        if (prices.every(isMetered)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `product ${row.id} has no licensed price in ${wanted.currency}`,
            });
        }

        return { product: row, seller: sold, prices };
    },
};

/** Sellers, checkouts and plan changes through the payment provider, refused without one. */
export class Billing {
    /** The payment provider, absent in a universe selling nothing. */
    readonly provider: PaymentProvider | undefined;
    /** The home finance keeps its rows in, the platform account's home in its residency. */
    readonly home: string;

    /** Bill through a provider, or through none, from a home. */
    constructor(provider: PaymentProvider | undefined, home: string) {
        this.provider = provider;
        this.home = home;
    }

    /** Decide whether an account is the platform's, whose home finance lives in. */
    isPlatform(accountId: Identifier<"account">): boolean {
        return Account.home(accountId) === this.home;
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

    /** Open the provider's checkout session for the mirrored customer. */
    async prepareCheckout(
        call: CallOf<typeof checkoutSession, "create">,
    ): Promise<PreparedCallOf<typeof checkoutSession, "create">["prepared"]> {
        // require a buyer without a subscription to the seller
        const { accountId } = call.input;
        const offering = await Offering.read(call, purchased(call.input));
        await Subscription.requireNone(call, accountId, offering.seller);
        const key = present(call.idempotencyKey, "a checkout's idempotency key");
        const provider = this.requireProvider();

        // expire the buyer's open session with the seller, refusing one the buyer paid
        const opened = await call.read(checkoutSession).rows({ accountId, status: "open" });
        const open = opened.find((row) => row.seller.id === offering.seller.id);
        if (
            open !== undefined &&
            (await provider.expire(open.providerId, `${key}/expire`)) === "complete"
        ) {
            throw new ServiceError("CONFLICT", {
                message: `checkout session ${open.id} is paid, and its subscription starts once the provider reports it`,
            });
        }

        // mirror the customer and the prices, and open the provider's session
        const customerId = await Billing.#mirrorCustomer(call, accountId, provider, key);
        const mirrored = await Billing.#mirrorPrices(offering, provider);
        const pages: CheckoutPages = {
            successUrl: call.input.successUrl,
            cancelUrl: call.input.cancelUrl,
        };
        const order = this.#order(offering, customerId, mirrored);
        const session = await provider.checkout(order, pages, `${key}/checkout`);

        return {
            customer: customerId,
            session,
            replaced: open === undefined ? null : open.id,
            mirrored,
        };
    }

    /** Keep a checkout session at the provider's page. */
    async openCheckout(
        call: PreparedCallOf<typeof checkoutSession, "create">,
        next: NextOf<typeof checkoutSession, "create">,
    ): Promise<CheckoutSession> {
        // record the buyer's customer and the prices mirrored
        const offering = await Offering.read(call, purchased(call.input));
        const { customer: customerId, session, replaced, mirrored } = call.prepared;
        await Billing.#record(call, call.input.accountId, customerId);
        await Billing.#recordMirror(call, offering, mirrored);

        // expire the replaced session unless the provider's event expired it first
        const earlier =
            replaced === null
                ? null
                : await call
                      .read(checkoutSession)
                      .row({ id: checkoutSession.identifier(replaced) });
        if (earlier !== null && earlier.status === "open") {
            await call.mutate(call.service, [checkoutSession.call("expire", { id: earlier.id })]);
        }

        // keep the session with its seller and page

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

    /** Expire a prepared session at the provider once its checkout rolled back. */
    async settleCheckout(
        call: SettlementCall,
        settled: Settled<PreparedCallOf<typeof checkoutSession, "create">["prepared"]>,
    ): Promise<void> {
        // leave a committed checkout, and one that prepared nothing
        if (settled.status === "committed" || settled.prepared === undefined) {
            return;
        }

        // expire the session it opened
        const session = settled.prepared.session.providerId;
        await this.requireProvider().expire(session, `${call.idempotencyKey}/rollback`);
    }

    /** Change a subscription's prices at the provider. */
    async prepareChange(call: CallOf<typeof subscription, "change">): Promise<PlanChange> {
        // close the current period onto the final invoice and cancel for the default product
        const offering = await Offering.read(call, purchased(call.input));
        const current = call.requireTarget();
        const key = present(call.idempotencyKey, "a change's idempotency key");
        const provider = this.requireProvider();
        const customerId = await Billing.#mirrorCustomer(call, current.accountId, provider, key);
        if (offering.product.isDefault) {
            const period = { start: current.currentPeriodStart, end: current.currentPeriodEnd };
            const invoice = { customer: customerId, providerId: null };
            const closed = await Charge.bill(call.view(), current, period, invoice, provider, key);
            await provider.cancel(current.providerId, `${key}/cancel`);

            return { kind: "cancel", customer: customerId, closed };
        }

        // mirror the prices and change the subscription's
        const mirrored = await Billing.#mirrorPrices(offering, provider);
        const order = this.#order(offering, customerId, mirrored);
        const changed = await provider.change(current.providerId, order.prices, `${key}/change`);

        return { kind: "change", customer: customerId, subscription: changed, mirrored };
    }

    /** Change a subscription's plan as prepared: new prices, or close for the default product. */
    async change(call: PreparedCallOf<typeof subscription, "change">): Promise<Subscription> {
        // close the period for the default product, the provider's webhook canceling the subscription
        const work = call.prepared;
        const current = call.requireTarget();
        await Billing.#record(call, current.accountId, work.customer);
        if (work.kind === "cancel") {
            const period = { start: current.currentPeriodStart, end: current.currentPeriodEnd };
            await Charge.close(call, current, period, work.closed);

            return current;
        }

        // follow the provider's change, with the prices mirrored and the metered prices
        const offering = await Offering.read(call, purchased(call.input));
        await Billing.#recordMirror(call, offering, work.mirrored);
        await call.mutate(call.service, [
            subscription.call("sync", { id: current.id, ...work.subscription }),
        ]);
        await SubscriptionItem.replaceMetered(call, current.id, offering.prices.filter(isMetered));

        return present(
            await call.read(subscription).row({ id: current.id }),
            `subscription ${current.id}`,
        );
    }

    /** Add an ended period's lines to the provider's draft invoice of its renewal. */
    prepareClose(call: CallOf<typeof subscription, "closePeriod">): Promise<PeriodClose> {
        // bill the period's lines onto the draft under the close's key
        const { periodStart, periodEnd, invoice } = call.input;
        const key = present(call.idempotencyKey, "a close's idempotency key");
        const period = { start: periodStart, end: periodEnd };

        return Charge.bill(
            call.view(),
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
        const offering = await Offering.read(call, purchased(session));
        const metered = offering.prices
            .filter(isMetered)
            .map((each) => ({ price: each, quantity: 1, providerId: null }));
        const billed = [...(await SubscriptionItem.prices(call, provided.items)), ...metered];
        const started = await Subscription.start(
            call,
            { accountId: session.accountId, seller: session.seller },
            billed,
            provided,
        );

        // complete the session with it
        const [, completed] = await call.mutate(call.service, [
            checkoutSession.call("record", { id: session.id, subscriptionId: started.id }),
            checkoutSession.call("complete", { id: session.id }),
        ]);

        return completed;
    }

    /** Record the provider's identifier of the buying account's customer. */
    static async #record(
        call: Call,
        accountId: Identifier<"account">,
        providerId: string,
    ): Promise<void> {
        const [buyer] = await call.read(customer).rows({ accountId });
        const id = present(buyer, `the customer of account ${accountId}`).id;
        await call.mutate(call.service, [customer.call("record", { id, providerId })]);
    }

    /** Record the provider's identifiers of the product and prices a sale mirrored first. */
    static async #recordMirror(
        call: Call,
        offering: Offering,
        mirrored: PriceMirror,
    ): Promise<void> {
        const recorded = mirrored.product === null ? [] : [mirrored.product];
        await call.mutate(call.service, [
            ...recorded.map((providerId) =>
                product.call("record", { id: offering.product.id, providerId }),
            ),
            ...Object.entries(mirrored.prices).map(([id, providerId]) =>
                price.call("record", { id: price.identifier(id), providerId }),
            ),
        ]);
    }

    /** Mirror an offering's product and licensed prices the provider lacks, keyed by their own. */
    static async #mirrorPrices(
        offering: Offering,
        provider: PaymentProvider,
    ): Promise<PriceMirror> {
        // mirror nothing once every licensed price is mirrored
        const sold = offering.product;
        const unmirrored = offering.prices.filter(
            (each) => !isMetered(each) && each.providerId === null,
        );
        if (unmirrored.length === 0) {
            return { product: null, prices: {} };
        }

        // mirror the product once, then each price on it
        const productId =
            sold.providerId === null
                ? await provider.createProduct(sold, `product/${sold.id}`)
                : sold.providerId;
        const prices: Record<string, string> = {};
        for (const each of unmirrored) {
            prices[each.id] = await provider.createPrice(productId, each, `price/${each.id}`);
        }

        return { product: sold.providerId === null ? productId : null, prices };
    }

    /** Keep the buying account's billing details at the provider, returning its identifier. */
    static async #mirrorCustomer(
        call: Call,
        accountId: Identifier<"account">,
        provider: PaymentProvider,
        key: string,
    ): Promise<string> {
        const [buyer] = await call.read(customer).rows({ accountId });

        return provider.upsertCustomer(
            present(buyer, `the customer of account ${accountId}`),
            `${key}/customer`,
        );
    }

    /** Read the order of a checkout or change: its seller's destination and its licensed prices. */
    #order(offering: Offering, customerId: string, mirrored: PriceMirror): Order {
        // sell on the provider account itself for the platform, or require an active connected one
        const sold = offering.seller;
        const destination = this.isPlatform(sold.accountId) ? null : Billing.#connected(sold);

        // pair each licensed price with the provider's identifier, kept or just mirrored
        const prices = offering.prices
            .filter((each) => !isMetered(each))
            .map((each) => ({
                price: each.providerId ?? present(mirrored.prices[each.id], `price ${each.id}`),
                quantity: 1,
            }));

        return { customer: customerId, destination, prices };
    }

    /** Require a seller's onboarded connected account, which takes the charges. */
    static #connected(sold: Seller): string {
        if (sold.providerId === null || sold.status !== "active") {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `seller ${sold.id} takes no charges yet`,
            });
        }

        return sold.providerId;
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
    return row.recurring.usage === "metered";
}

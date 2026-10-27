import { Account } from "@destack/account/object";
import { FinanceError } from "../error/error.ts";
import type {
    MethodContext,
    MethodContextOf,
    NextOf,
    Settled,
    SettlementContext,
} from "@destack/object";
import type { Snapshot } from "@destack/db";
import { ObjectReference } from "@destack/sync";
import { found, type Identifier, present, zip } from "@destack/schema";
import type { Period } from "../rate/period.ts";
import type { CheckoutPages, Order, PaymentProvider } from "../provider/provider.ts";
import { Cost } from "../rate/amount.ts";
import {
    charge,
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
import type {} from "@destack/package/import-meta";

/** A product with its recurring prices in one currency. */
interface Offering {
    /** The product. */
    readonly product: Product;
    /** The seller of the product's account. */
    readonly seller: Seller;
    /** The product's active recurring prices in the currency, empty for a default product. */
    readonly prices: readonly Price[];
}

/** The offerings buyers subscribe to. */
const Offering = {
    /** Read a product's offering in a currency, a default product's without prices. */
    async read(wanted: Purchasable, context: Pick<MethodContext, "read">): Promise<Offering> {
        // read the active product
        const row = await context
            .read(product)
            .findFirst({ where: { id: product.identifier(wanted.product.id) } });
        if (row === undefined || !row.active || row.scope !== wanted.product.scope) {
            throw FinanceError.of(
                "PRODUCT_UNAVAILABLE",
                `product ${wanted.product.id} is not on sale`,
            );
        }

        // read the seller of the product's account
        const [sold] = await context.read(seller).findMany({ where: { accountId: row.accountId } });
        if (sold === undefined) {
            throw FinanceError.of("SELLER_MISSING", `account ${row.accountId} is not a seller`);
        } else if (row.isDefault) {
            return { product: row, seller: sold, prices: [] };
        }

        // read its active recurring prices in the currency, requiring a licensed fee, 0 for pay-as-you-go
        const prices = await context
            .read(price)
            .findMany({ where: { parentId: row.id, currency: wanted.currency, active: true } });
        if (prices.every((each) => Price.isMetered(each))) {
            throw FinanceError.of(
                "LICENSED_PRICE_MISSING",
                `product ${row.id} has no licensed price in ${wanted.currency}`,
            );
        }

        return { product: row, seller: sold, prices };
    },
};

/** Sellers, checkouts and plan changes through the payment provider, refused without one. */
export class Billing {
    /** The payment provider, absent for a home billing through none. */
    readonly provider: PaymentProvider | undefined;
    /** The home finance stores its rows in, the platform account's home in its residency. */
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
            throw FinanceError.of(
                "PROVIDER_MISSING",
                "this home has no payment provider, so it cannot sell",
            );
        }

        return this.provider;
    }

    /** Open the provider's checkout session for the mirrored customer. */
    async prepareCheckout(
        context: MethodContextOf<typeof checkoutSession, "create", unknown>,
    ): Promise<MethodContextOf<typeof checkoutSession, "create">["prepared"]> {
        // require a buyer without a subscription to the seller
        const { accountId } = context.input;
        const offering = await Offering.read(Purchasable.of(context.input), context);
        await Subscription.requireNone(accountId, offering.seller, context);
        const key = present(context.idempotencyKey, "a checkout's idempotency key");
        const provider = this.requireProvider();

        // expire the buyer's open session with the seller, refusing one the buyer paid
        const opened = await context
            .read(checkoutSession)
            .findMany({ where: { accountId, status: "open" } });
        const open = opened.find((row) => row.sellerId === offering.seller.id);
        if (
            open !== undefined &&
            (await provider.expire(open.providerId, `${key}/expire`)) === "complete"
        ) {
            throw FinanceError.of(
                "CHECKOUT_COMPLETE",
                `checkout session ${open.id} is paid, and its subscription starts once the provider reports it`,
            );
        }

        // mirror the customer and the prices, and open the provider's session
        const customerId = await Billing.#mirrorCustomer(accountId, provider, key, context);
        const mirrored = await Billing.#mirrorPrices(offering, provider);
        const pages: CheckoutPages = {
            successUrl: context.input.successUrl,
            cancelUrl: context.input.cancelUrl,
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

    /** Open a checkout session at the provider's page. */
    async openCheckout(
        context: MethodContextOf<typeof checkoutSession, "create">,
        next: NextOf<typeof checkoutSession, "create">,
    ): Promise<CheckoutSession> {
        // record the buyer's customer and the prices mirrored
        const offering = await Offering.read(Purchasable.of(context.input), context);
        const { customer: customerId, session, replaced, mirrored } = context.prepared;
        await Billing.#record(context.input.accountId, customerId, context);
        await Billing.#recordMirror(offering, mirrored, context);

        // expire the replaced session unless the provider's event expired it first
        const earlier =
            replaced === null
                ? undefined
                : await context
                      .read(checkoutSession)
                      .findFirst({ where: { id: checkoutSession.identifier(replaced) } });
        if (earlier !== undefined && earlier.status === "open") {
            await context.mutate(context.self, [
                checkoutSession.call("expire", { id: earlier.id }),
            ]);
        }

        return next(
            context.with({
                input: {
                    ...context.input,
                    ...ObjectReference.columns(
                        "seller",
                        seller.reference(offering.seller.scope, offering.seller.id),
                    ),
                    url: session.url,
                    providerId: session.providerId,
                },
            }),
        );
    }

    /** Expire a prepared session at the provider once its checkout rolled back. */
    async settleCheckout(
        context: SettlementContext,
        settled: Settled<MethodContextOf<typeof checkoutSession, "create">["prepared"]>,
    ): Promise<void> {
        // leave a committed checkout, and one that prepared nothing
        if (settled.status === "committed" || settled.prepared === undefined) {
            return;
        }

        // expire the session it opened
        const session = settled.prepared.session.providerId;
        await this.requireProvider().expire(session, `${context.idempotencyKey}/rollback`);
    }

    /** Change a subscription's prices, or close its period for the default product. */
    async prepareChange(
        context: MethodContextOf<typeof subscription, "change", unknown>,
    ): Promise<PlanChange> {
        // close the current period onto the final invoice for the default product
        const { currency } = context.input;
        const offering = await Offering.read({ product: context.input.product, currency }, context);
        const current = context.target;
        const key = present(context.idempotencyKey, "a change's idempotency key");
        const provider = this.requireProvider();
        const customerId = await Billing.#mirrorCustomer(current.accountId, provider, key, context);
        if (offering.product.isDefault) {
            const period = { start: current.currentPeriodStart, end: current.currentPeriodEnd };
            const invoice = { customer: customerId, providerId: null };
            const closed = await Billing.#bill(current, period, invoice, provider, key, context);

            return {
                kind: "cancel",
                customer: customerId,
                subscription: current.providerId,
                closed,
            };
        }

        // mirror the prices and change the subscription's
        const mirrored = await Billing.#mirrorPrices(offering, provider);
        const order = this.#order(offering, customerId, mirrored);

        return {
            kind: "change",
            customer: customerId,
            subscription: current.providerId,
            prices: [...order.prices],
            mirrored,
        };
    }

    /** Change or cancel a subscription at the provider once its change committed. */
    async settleChange(context: SettlementContext, settled: Settled<PlanChange>): Promise<void> {
        // leave a rolled back change
        if (settled.status === "rolledBack") {
            return;
        }

        // change the prices, or cancel for the default product
        const work = settled.prepared;
        const provider = this.requireProvider();
        if (work.kind === "change") {
            await provider.change(
                work.subscription,
                work.prices,
                `${context.idempotencyKey}/change`,
            );
        } else {
            await provider.cancel(work.subscription, `${context.idempotencyKey}/cancel`);
        }
    }

    /** Change a subscription's plan as prepared: new prices, or close for the default product. */
    async change(context: MethodContextOf<typeof subscription, "change">): Promise<Subscription> {
        // close the period for the default product, the provider's webhook canceling the subscription
        const work = context.prepared;
        const current = context.target;
        await Billing.#record(current.accountId, work.customer, context);
        if (work.kind === "cancel") {
            const period = { start: current.currentPeriodStart, end: current.currentPeriodEnd };
            await Billing.#invoice(current, period, work.closed, context);

            return current;
        }

        // record the mirrored prices and replace the metered ones, as the provider's event syncs the rest
        const { currency } = context.input;
        const offering = await Offering.read({ product: context.input.product, currency }, context);
        await Billing.#recordMirror(offering, work.mirrored, context);
        await SubscriptionItem.replaceMetered(
            current.id,
            offering.prices.filter((each) => Price.isMetered(each)),
            context,
        );

        return present(
            await context.read(subscription).findFirst({ where: { id: current.id } }),
            `subscription ${current.id}`,
        );
    }

    /** Add an ended period's lines to the provider's draft invoice of its renewal. */
    prepareClose(
        context: MethodContextOf<typeof subscription, "closePeriod", unknown>,
    ): Promise<PeriodClose> {
        // bill the period's lines onto the draft under the close's key
        const { periodStart, periodEnd, invoice } = context.input;
        const key = present(context.idempotencyKey, "a close's idempotency key");
        const period = { start: periodStart, end: periodEnd };

        return Billing.#bill(context.target, period, invoice, this.requireProvider(), key, context);
    }

    /** Close an ended period as prepared. */
    static async close(
        context: MethodContextOf<typeof subscription, "closePeriod">,
    ): Promise<Subscription> {
        const row = context.target;
        await Billing.#invoice(
            row,
            { start: context.input.periodStart, end: context.input.periodEnd },
            context.prepared,
            context,
        );

        return row;
    }

    /** Settle a session with the subscription the provider started, with its metered prices. */
    static async settle(
        context: MethodContextOf<typeof checkoutSession, "settle">,
    ): Promise<CheckoutSession> {
        // start the subscription to the provider's items and the offering's metered prices
        const session = context.target;
        const provided = context.input.subscription;
        const offering = await Offering.read(Purchasable.of(session), context);
        const metered = offering.prices
            .filter((each) => Price.isMetered(each))
            .map((each) => ({ price: each, quantity: 1, providerId: null }));
        const billed = [...(await SubscriptionItem.prices(provided.items, context)), ...metered];
        const started = await Subscription.start(
            { accountId: session.accountId, seller: ObjectReference.at("seller", session) },
            billed,
            provided,
            context,
        );

        // complete the session with it
        const [completed] = await context.mutate(context.self, [
            checkoutSession.call("complete", { id: session.id, subscriptionId: started.id }),
        ]);

        return completed;
    }

    /** Record the provider's identifier of the buying account's customer. */
    static async #record(
        accountId: Identifier<"account">,
        providerId: string,
        context: MethodContext,
    ): Promise<void> {
        const [buyer] = await context.read(customer).findMany({ where: { accountId } });
        const id = present(buyer, `the customer of account ${accountId}`).id;
        await context.mutate(context.self, [customer.call("record", { id, providerId })]);
    }

    /** Record the provider's identifiers of the product and prices a sale mirrored first. */
    static async #recordMirror(
        offering: Offering,
        mirrored: PriceMirror,
        context: MethodContext,
    ): Promise<void> {
        const recorded = mirrored.product === null ? [] : [mirrored.product];
        await context.mutate(context.self, [
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
            (each) => !Price.isMetered(each) && each.providerId === null,
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

    /** Store the buying account's billing details at the provider, returning its identifier. */
    static async #mirrorCustomer(
        accountId: Identifier<"account">,
        provider: PaymentProvider,
        key: string,
        context: MethodContext,
    ): Promise<string> {
        const [buyer] = await context.read(customer).findMany({ where: { accountId } });

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

        // pair each licensed price with the provider's identifier, stored or just mirrored
        const prices = offering.prices
            .filter((each) => !Price.isMetered(each))
            .map((each) => ({
                price: each.providerId ?? present(mirrored.prices[each.id], `price ${each.id}`),
                quantity: 1,
            }));

        return { customer: customerId, destination, prices };
    }

    /** Require a seller's onboarded connected account, which takes the charges. */
    static #connected(sold: Seller): string {
        if (sold.providerId === null || sold.status !== "active") {
            throw FinanceError.of(
                "SELLER_INACTIVE",
                `seller ${sold.id} is ${sold.status}: only an active seller can take charges`,
            );
        }

        return sold.providerId;
    }

    /** Add a period's open lines and their drawn included usage to a provider invoice. */
    static async #bill(
        row: Subscription,
        period: Period,
        invoice: { readonly customer: string; readonly providerId: string | null },
        provider: PaymentProvider,
        key: string,
        reader: Pick<Snapshot, "read">,
    ): Promise<PeriodClose> {
        // draw the period's included usage on its open usage lines
        const { lines, amounts, total, currency, includedUsage } = await Charge.draw(
            row,
            period.start,
            reader,
        );
        const drawn = Math.min(includedUsage, total);

        // add the lines billing an amount at once, each under its own key
        const billed = {
            customer: invoice.customer,
            invoice: invoice.providerId,
            currency,
            period,
        };
        const providerLines = await Promise.all(
            zip(lines, amounts).map(async ([line, amount]) =>
                amount > 0
                    ? provider.bill(
                          {
                              ...billed,
                              amount,
                              description: Charge.describe(line),
                              charge: line.id,
                          },
                          `${key}/${line.id}`,
                      )
                    : null,
            ),
        );
        const added = Object.fromEntries(
            zip(lines, providerLines).map(([line, providerLine]) => [line.id, providerLine]),
        );

        // add the credit for what was drawn
        const credit = {
            ...billed,
            amount: -drawn,
            description: "Included usage",
            charge: null,
        };
        const creditLine = drawn === 0 ? null : await provider.bill(credit, `${key}/credit`);

        return { lines: added, drawn, creditLine };
    }

    /** Mark a period's lines invoiced exactly as billed, and record their drawn included usage. */
    static async #invoice(
        row: Subscription,
        period: Period,
        closed: PeriodClose,
        context: MethodContext,
    ): Promise<void> {
        // refuse a period whose open lines differ from those billed
        const { lines, currency } = await Charge.draw(row, period.start, context);
        const billed = new Map(Object.entries(closed.lines));
        if (lines.length !== billed.size || lines.some((line) => !billed.has(line.id))) {
            throw FinanceError.of(
                "USAGE_CHANGED",
                `the usage lines of subscription ${row.id} changed since its period was billed`,
            );
        }

        // mark each billed line invoiced, recording the included usage drawn as a negative credit line
        const invoiced = lines.map((line) =>
            charge.call("update", {
                id: line.id,
                status: "invoiced",
                providerId: found(billed, line.id),
            }),
        );
        const drawn = Cost.of(-Cost.whole(closed.drawn));
        const credited =
            closed.drawn > 0
                ? [
                      charge.call("create", {
                          accountId: row.accountId,
                          ...ObjectReference.columns("seller", ObjectReference.at("seller", row)),
                          chargeCategory: "Credit",
                          chargePeriodStart: period.start,
                          chargePeriodEnd: period.end,
                          billingCurrency: currency,
                          listCost: "0",
                          billedCost: drawn,
                          effectiveCost: drawn,
                          status: "invoiced",
                          providerId: closed.creditLine,
                      }),
                  ]
                : [];
        await context.mutate(context.self, [...invoiced, ...credited]);
    }
}

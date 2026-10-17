import type { Fetch } from "@destack/runtime";
import { schema } from "@destack/schema";
import type {
    CheckoutPages,
    InvoiceLine,
    Order,
    PaymentProvider,
    ProviderEvent,
} from "../provider.ts";
import { type Customer, type Price, Product } from "../../object/index.ts";
import type { Secret } from "@destack/vault";
import { StripeClient, type StripeParameter } from "./client.ts";
import {
    Stripe,
    StripeAccount,
    StripeCheckoutSession,
    StripeCustomer,
    StripeEvent,
    StripeObject,
    StripeOwner,
} from "./object.ts";

/** The metadata key tagging customers and connected accounts with their finance's home. */
const HOME_KEY = "destack_home";

/** The Checkout events finance follows. */
const CHECKOUT_EVENTS = ["checkout.session.completed", "checkout.session.expired"] as const;

/** The subscription events finance follows. */
const SUBSCRIPTION_EVENTS: ReadonlySet<string> = new Set([
    "customer.subscription.updated",
    "customer.subscription.deleted",
    "customer.subscription.paused",
    "customer.subscription.resumed",
]);

/** The invoice events finance mirrors. */
const INVOICE_EVENTS: ReadonlySet<string> = new Set([
    "invoice.created",
    "invoice.finalized",
    "invoice.paid",
    "invoice.payment_failed",
    "invoice.voided",
    "invoice.marked_uncollectible",
]);

/** The event types finance follows, which each home's webhook endpoint subscribes to. */
export const STRIPE_EVENTS: readonly string[] = [
    ...CHECKOUT_EVENTS,
    ...SUBSCRIPTION_EVENTS,
    ...INVOICE_EVENTS,
    "account.updated",
];

/** The key and home a Stripe provider runs with. */
export interface StripeProviderOptions {
    /** The platform account's secret API key, read through the vault. */
    readonly key: Secret;
    /** The home whose finance bills through the provider, stored on its customers and sellers. */
    readonly home: string;
    /** The fetch calling Stripe. */
    readonly fetch: Fetch;
}

/** Finance's payment provider on Stripe. */
export class StripeProvider implements PaymentProvider {
    /** The provider's name, which keys each of its events' redeliveries. */
    readonly name = "stripe";
    /** The API client. */
    readonly #client: StripeClient;
    /** The options. */
    readonly #options: StripeProviderOptions;

    /** Bill through Stripe with keys read through the vault. */
    constructor(options: StripeProviderOptions) {
        this.#client = new StripeClient(options.key, options.fetch);
        this.#options = options;
    }

    /** Open an Express connected account taking card billing and transfers. */
    async openSeller(country: string, key: string): Promise<string> {
        const account = await this.#client.request(
            "POST",
            "/v1/accounts",
            {
                type: "express",
                country,
                metadata: { [HOME_KEY]: this.#options.home },
                capabilities: {
                    card_payments: { requested: true },
                    transfers: { requested: true },
                },
            },
            key,
        );

        return StripeAccount.parse(account).id;
    }

    /** Open an account link onboarding a connected account. */
    async onboard(
        seller: string,
        pages: { readonly returnUrl: string; readonly refreshUrl: string },
    ): Promise<string> {
        const link = await this.#client.request("POST", "/v1/account_links", {
            account: seller,
            type: "account_onboarding",
            return_url: pages.returnUrl,
            refresh_url: pages.refreshUrl,
        });

        return schema.looseObject({ url: schema.string() }).parse(link).url;
    }

    /** Create or update a customer of the platform account. */
    async upsertCustomer(customer: Customer, key: string): Promise<string> {
        // write the details invoices show
        const details = {
            email: customer.email,
            name: customer.name,
            address:
                customer.address === null
                    ? undefined
                    : {
                          line1: customer.address.line1,
                          line2: customer.address.line2,
                          city: customer.address.city,
                          postal_code: customer.address.postalCode,
                          state: customer.address.state,
                          country: customer.address.country,
                      },
            metadata: { account: customer.accountId, [HOME_KEY]: this.#options.home },
        };
        const path =
            customer.providerId === null ? "/v1/customers" : `/v1/customers/${customer.providerId}`;
        const answered = await this.#client.request("POST", path, details, key);

        return StripeObject.parse(answered).id;
    }

    /** Create a product once per key. */
    createProduct(product: Product, key: string): Promise<string> {
        return this.#create("/v1/products", key, {
            ...Product.render(product),
            metadata: { product: product.id },
        });
    }

    /** Create a licensed price of a mirrored product once per key. */
    createPrice(product: string, price: Price, key: string): Promise<string> {
        const recurring = price.recurring;

        return this.#create("/v1/prices", key, {
            product,
            currency: price.currency.toLowerCase(),
            unit_amount: price.unitAmount,
            unit_amount_decimal: price.unitAmountDecimal,
            billing_scheme: price.billingScheme,
            tiers_mode: price.tiers === null ? undefined : "graduated",
            tiers: price.tiers?.map((tier) => ({
                up_to: tier.upTo,
                unit_amount: tier.unitAmount,
                flat_amount: tier.flatAmount,
            })),
            recurring:
                recurring === null
                    ? undefined
                    : {
                          interval: recurring.interval,
                          interval_count: recurring.intervalCount,
                          usage_type: "licensed",
                      },
        });
    }

    /** Open a Checkout session paying the seller, taxed by Stripe Tax. */
    async checkout(
        order: Order,
        pages: CheckoutPages,
        key: string,
    ): Promise<{ readonly providerId: string; readonly url: string }> {
        // transfer a connected seller's charges, and leave the platform's own
        const transfer =
            order.destination === null ? {} : { transfer_data: { destination: order.destination } };
        const session = await this.#client.request(
            "POST",
            "/v1/checkout/sessions",
            {
                mode: "subscription",
                customer: order.customer,
                line_items: order.prices.map(({ price, quantity }) => ({ price, quantity })),
                subscription_data: transfer,
                automatic_tax: { enabled: true },
                customer_update: { address: "auto" },
                success_url: pages.successUrl,
                cancel_url: pages.cancelUrl,
            },
            key,
        );

        return schema
            .looseObject({ id: schema.string(), url: schema.string() })
            .transform(({ id, url }) => ({ providerId: id, url }))
            .parse(session);
    }

    /** Replace a subscription's items with the prices, prorating the licensed ones. */
    async change(subscription: string, prices: Order["prices"], key: string): Promise<void> {
        // read the current items
        const current = Stripe.subscription(await this.#fetch("subscriptions", subscription));

        // delete them and add the new prices
        const items: StripeParameter[] = [
            ...current.items.map((item) => ({ id: item.providerId, deleted: true })),
            ...prices.map(({ price, quantity }) => ({ price, quantity })),
        ];
        await this.#client.request(
            "POST",
            `/v1/subscriptions/${subscription}`,
            { items, proration_behavior: "create_prorations" },
            key,
        );
    }

    /** Expire an open Checkout session, or report it complete once its buyer paid. */
    async expire(session: string, key: string): Promise<"expired" | "complete"> {
        // leave a session its buyer paid, or one that expired already
        const read = StripeCheckoutSession.parse(await this.#fetch("checkout/sessions", session));
        if (read.status !== "open") {
            return read.status;
        }

        // expire the open one
        await this.#client.request("POST", `/v1/checkout/sessions/${session}/expire`, {}, key);

        return "expired";
    }

    /** Cancel a subscription now, invoicing its usage and prorations. */
    async cancel(subscription: string, key: string): Promise<void> {
        await this.#client.request(
            "DELETE",
            `/v1/subscriptions/${subscription}`,
            { invoice_now: true, prorate: true },
            key,
        );
    }

    /** Add an invoice item to a draft invoice, once per idempotency key. */
    async bill(line: InvoiceLine, key: string): Promise<string> {
        return this.#create("/v1/invoiceitems", key, {
            customer: line.customer,
            invoice: line.invoice ?? undefined,
            amount: line.amount,
            currency: line.currency.toLowerCase(),
            description: line.description,
            period: {
                start: Math.floor(line.period.start / 1000),
                end: Math.floor(line.period.end / 1000),
            },
            metadata: line.charge === null ? undefined : { charge: line.charge },
        });
    }

    /** Read a webhook delivery's event as Stripe has it, absent for one finance here ignores. */
    async receive(request: Request): Promise<ProviderEvent | undefined> {
        // fetch the delivery's event
        const { id } = StripeObject.parse(await request.json());
        const event = StripeEvent.parse(await this.#fetch("events", id));

        // leave an event of another home's customer or seller
        if (!(await this.#isHome(event.data.object))) {
            return undefined;
        }

        return this.#read(event);
    }

    /** Report whether an object belongs to a customer or connected account of this home. */
    async #isHome(object: unknown): Promise<boolean> {
        // read a connected account's tag
        const owner = StripeOwner.parse(object);
        if (owner.object === "account") {
            return StripeAccount.parse(object).metadata[HOME_KEY] === this.#options.home;
        }
        // read its customer's tag
        else if (owner.customer !== undefined && owner.customer !== null) {
            const customer = StripeCustomer.parse(await this.#fetch("customers", owner.customer));

            return customer.metadata[HOME_KEY] === this.#options.home;
        }

        return false;
    }

    /** Read what an event tells finance, fetching its current subscription, invoice or account. */
    async #read(event: schema.Infer<typeof StripeEvent>): Promise<ProviderEvent | undefined> {
        // read the event's object and its identity, then what it tells by its type
        const object = event.data.object;
        const delivered = { id: event.id, time: event.created * 1000 };
        if (event.type === "checkout.session.completed") {
            const session = StripeCheckoutSession.parse(object);
            const subscription =
                session.subscription === null
                    ? null
                    : Stripe.subscription(await this.#fetch("subscriptions", session.subscription));

            return {
                ...delivered,
                kind: "checkout",
                session: session.id,
                status: "complete",
                subscription,
            };
        } else if (event.type === "checkout.session.expired") {
            const session = StripeCheckoutSession.parse(object);

            return {
                ...delivered,
                kind: "checkout",
                session: session.id,
                status: "expired",
                subscription: null,
            };
        } else if (SUBSCRIPTION_EVENTS.has(event.type)) {
            const { id } = StripeObject.parse(object);

            return {
                ...delivered,
                kind: "subscription",
                subscription: Stripe.subscription(await this.#fetch("subscriptions", id)),
            };
        } else if (INVOICE_EVENTS.has(event.type)) {
            const { id } = StripeObject.parse(object);

            return {
                ...delivered,
                kind: "invoice",
                invoice: Stripe.invoice(await this.#fetch("invoices", id)),
            };
        } else if (event.type === "account.updated") {
            const { id } = StripeObject.parse(object);
            const account = StripeAccount.parse(await this.#fetch("accounts", id));

            return {
                ...delivered,
                kind: "seller",
                seller: account.id,
                chargesEnabled: account.charges_enabled,
                payoutsEnabled: account.payouts_enabled,
            };
        }

        return undefined;
    }

    /** Fetch an object of a kind as Stripe has it now. */
    #fetch(
        kind:
            | "subscriptions"
            | "invoices"
            | "accounts"
            | "customers"
            | "events"
            | "checkout/sessions",
        id: string,
    ): Promise<unknown> {
        return this.#client.request("GET", `/v1/${kind}/${id}`);
    }

    /** Create an object once per idempotency key, returning its identifier. */
    async #create(
        path: string,
        key: string,
        parameters: Readonly<Record<string, StripeParameter>>,
    ): Promise<string> {
        const created = await this.#client.request("POST", path, parameters, key);

        return StripeObject.parse(created).id;
    }
}

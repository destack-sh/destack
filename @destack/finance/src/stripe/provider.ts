import type { Fetch } from "@destack/service";
import { schema } from "@destack/schema";
import type { InvoiceLine, Order, PaymentProvider, ProviderEvent } from "../provider/provider.ts";
import type { Customer, PriceTerms, Product, ProviderSubscription } from "../object/index.ts";
import { StripeClient, type StripeParameter, type StripeSecret } from "./client.ts";
import { Stripe, StripeAccount, StripeCheckoutSession, StripeEvent } from "./object.ts";
import { STRIPE_SIGNATURE_HEADER, StripeSignature } from "./signature.ts";

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

/** The keys and pages a Stripe provider runs with. */
export interface StripeProviderOptions {
    /** The platform account's secret API key, read through the vault. */
    readonly key: StripeSecret;
    /** The webhook endpoint's signing secret, read through the vault. */
    readonly webhookSecret: StripeSecret;
    /** The fetch reaching Stripe. */
    readonly fetch: Fetch;
    /** The pages Checkout returns buyers to. */
    readonly pages: { readonly successUrl: string; readonly cancelUrl: string };
    /** Read the current time, in UTC epoch milliseconds, the system clock by default. */
    readonly now?: () => number;
}

/** Finance's payment provider on Stripe. */
export class StripeProvider implements PaymentProvider {
    /** The provider's name, which its webhook path ends with. */
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
            metadata: { account: customer.scope },
        };
        const path =
            customer.providerId === null ? "/v1/customers" : `/v1/customers/${customer.providerId}`;
        const answered = await this.#client.request("POST", path, details, key);

        return schema.looseObject({ id: schema.string() }).parse(answered).id;
    }

    /** Create the product once and the licensed price. */
    async createPrice(
        product: Product,
        price: PriceTerms,
        key: string,
    ): Promise<{ readonly product: string; readonly price: string }> {
        // mirror the product once
        const productId =
            product.providerId ??
            (await this.#create("/v1/products", `${key}/product`, {
                name: product.name,
                description: product.description,
                metadata: { product: product.id },
            }));

        // mirror the price on the product
        const recurring = price.recurring;
        const priceId = await this.#create("/v1/prices", `${key}/price`, {
            product: productId,
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

        return { product: productId, price: priceId };
    }

    /** Open a Checkout session paying the seller, taxed by Stripe Tax. */
    async checkout(
        order: Order,
        key: string,
    ): Promise<{ readonly providerId: string; readonly url: string }> {
        const session = await this.#client.request(
            "POST",
            "/v1/checkout/sessions",
            {
                mode: "subscription",
                customer: order.customer,
                line_items: order.prices.map(({ price, quantity }) => ({ price, quantity })),
                subscription_data: { transfer_data: { destination: order.seller } },
                automatic_tax: { enabled: true },
                customer_update: { address: "auto" },
                success_url: this.#options.pages.successUrl,
                cancel_url: this.#options.pages.cancelUrl,
            },
            key,
        );

        return schema
            .looseObject({ id: schema.string(), url: schema.string() })
            .transform(({ id, url }) => ({ providerId: id, url }))
            .parse(session);
    }

    /** Replace a subscription's items with the prices, prorating the licensed ones. */
    async change(
        subscription: string,
        prices: Order["prices"],
        key: string,
    ): Promise<ProviderSubscription> {
        // read the current items
        const current = Stripe.subscription(
            await this.#client.request("GET", `/v1/subscriptions/${subscription}`),
        );

        // delete them and add the new prices
        const items: StripeParameter[] = [
            ...current.items.map((item) => ({ id: item.providerId, deleted: true })),
            ...prices.map(({ price, quantity }) => ({ price, quantity })),
        ];
        const changed = await this.#client.request(
            "POST",
            `/v1/subscriptions/${subscription}`,
            { items, proration_behavior: "create_prorations" },
            key,
        );

        return Stripe.subscription(changed);
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
            metadata: { charge: line.charge },
        });
    }

    /** Verify a webhook delivery's signature and read the event it carries. */
    async receive(request: Request): Promise<ProviderEvent | undefined> {
        // verify the signature over the raw payload
        const payload = await request.text();
        const secret = await StripeClient.text(this.#options.webhookSecret);
        const now = this.#options.now?.() ?? Date.now();
        await StripeSignature.verify(
            payload,
            request.headers.get(STRIPE_SIGNATURE_HEADER),
            secret,
            now,
        );

        return this.#read(StripeEvent.parse(JSON.parse(payload)));
    }

    /** Read what an event tells finance, fetching a completed session's subscription. */
    async #read(event: schema.Infer<typeof StripeEvent>): Promise<ProviderEvent | undefined> {
        const object = event.data.object;
        if (event.type === "checkout.session.completed") {
            const session = StripeCheckoutSession.parse(object);
            const subscription =
                session.subscription === null
                    ? null
                    : Stripe.subscription(
                          await this.#client.request(
                              "GET",
                              `/v1/subscriptions/${session.subscription}`,
                          ),
                      );

            return { kind: "checkout", session: session.id, status: "complete", subscription };
        } else if (event.type === "checkout.session.expired") {
            const session = StripeCheckoutSession.parse(object);

            return { kind: "checkout", session: session.id, status: "expired", subscription: null };
        } else if (SUBSCRIPTION_EVENTS.has(event.type)) {
            return { kind: "subscription", subscription: Stripe.subscription(object) };
        } else if (INVOICE_EVENTS.has(event.type)) {
            return { kind: "invoice", invoice: Stripe.invoice(object) };
        } else if (event.type === "account.updated") {
            const account = StripeAccount.parse(object);

            return {
                kind: "seller",
                seller: account.id,
                chargesEnabled: account.charges_enabled,
                payoutsEnabled: account.payouts_enabled,
            };
        }

        return undefined;
    }

    /** Create an object once per idempotency key, returning its identifier. */
    async #create(
        path: string,
        key: string,
        parameters: Readonly<Record<string, StripeParameter>>,
    ): Promise<string> {
        const created = await this.#client.request("POST", path, parameters, key);

        return schema.looseObject({ id: schema.string() }).parse(created).id;
    }
}

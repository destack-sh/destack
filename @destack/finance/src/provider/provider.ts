import type { Call } from "@destack/sync";
import { Snapshot, Transaction } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { present } from "@destack/schema";
import { CallError } from "@destack/object";
import { RequestId } from "@destack/sync";
import {
    checkoutSession,
    customer,
    type Customer,
    invoice,
    type Invoice,
    type Price,
    type Product,
    type ProviderSubscription,
    seller,
    subscription,
} from "../object/index.ts";

/** An invoice as the payment provider issued it, with the customer and subscription it bills. */
export type ProviderInvoice = Pick<
    Invoice,
    | "number"
    | "status"
    | "currency"
    | "subtotal"
    | "tax"
    | "total"
    | "amountPaid"
    | "periodStart"
    | "periodEnd"
    | "hostedUrl"
    | "pdfUrl"
    | "issuedAt"
    | "providerId"
> & {
    /** The provider's identifier of the billed customer. */
    readonly customer: string;
    /** The provider's identifier of the billed subscription, null without a subscription. */
    readonly subscription: string | null;
    /** Whether the invoice starts a billing period, closing the period before it. */
    readonly isPeriodStart: boolean;
};

/** What a provider's webhook delivery tells finance, as the provider has it now. */
export type ProviderEvent = {
    /** The provider's identifier of the event, which a redelivery repeats. */
    readonly id: string;
    /** When the provider recorded the event, in UTC epoch milliseconds. */
    readonly time: number;
} & (
    | {
          /** A checkout ended. */
          readonly kind: "checkout";
          /** The provider's identifier of the session. */
          readonly session: string;
          /** Whether the buyer paid or let the session lapse. */
          readonly status: "complete" | "expired";
          /** The subscription a completed session started. */
          readonly subscription: ProviderSubscription | null;
      }
    | {
          /** A subscription changed. */
          readonly kind: "subscription";
          /** The subscription as the provider bills it now. */
          readonly subscription: ProviderSubscription;
      }
    | {
          /** An invoice was issued or changed. */
          readonly kind: "invoice";
          /** The invoice as the provider has it now. */
          readonly invoice: ProviderInvoice;
      }
    | {
          /** A seller's connected account changed. */
          readonly kind: "seller";
          /** The provider's identifier of the connected account. */
          readonly seller: string;
          /** Whether the account takes charges. */
          readonly chargesEnabled: boolean;
          /** Whether the account is paid out. */
          readonly payoutsEnabled: boolean;
      }
);

/** A purchase of a seller's recurring prices a customer pays at the provider's page. */
export interface Order {
    /** The provider's identifier of the buying customer. */
    readonly customer: string;
    /** The provider's identifier of the seller's connected account, null for the platform. */
    readonly destination: string | null;
    /** The provider's identifiers of the licensed prices, with the quantity of each. */
    readonly prices: readonly { readonly price: string; readonly quantity: number }[];
}

/** The pages a checkout returns its buyer to. */
export interface CheckoutPages {
    /** The page after a completed checkout. */
    readonly successUrl: string;
    /** The page after a canceled checkout. */
    readonly cancelUrl: string;
}

/** A priced line on a customer's invoice, such as a period's usage charge or its credit. */
export interface InvoiceLine {
    /** The provider's identifier of the customer. */
    readonly customer: string;
    /** The provider's identifier of the draft invoice, null for the customer's next invoice. */
    readonly invoice: string | null;
    /** The amount in whole minor units, negative for a credit. */
    readonly amount: number;
    /** The currency of the amount. */
    readonly currency: string;
    /** What the line bills, as the invoice shows it. */
    readonly description: string;
    /** The period the line bills, in UTC epoch milliseconds. */
    readonly period: { readonly start: number; readonly end: number };
    /** The charge the line bills, null for the included usage credit the close records. */
    readonly charge: string | null;
}

/** A payment provider finance bills through. */
export interface PaymentProvider {
    /** The provider's name, which keys each of its events' redeliveries. */
    readonly name: string;

    /** Open a connected account for a seller in a country, returning its identifier. */
    readonly openSeller: (country: string, key: string) => Promise<string>;
    /** Open the onboarding page of a seller's connected account. */
    readonly onboard: (
        seller: string,
        pages: { readonly returnUrl: string; readonly refreshUrl: string },
    ) => Promise<string>;
    /** Store a customer's billing details, returning the provider's identifier. */
    readonly upsertCustomer: (customer: Customer, key: string) => Promise<string>;
    /** Mirror a product once per key, returning the provider's identifier. */
    readonly createProduct: (product: Product, key: string) => Promise<string>;
    /** Mirror a licensed price of a mirrored product once per key, returning its identifier. */
    readonly createPrice: (product: string, terms: Price, key: string) => Promise<string>;

    /** Open a checkout of an order, returning the provider's session identifier and page. */
    readonly checkout: (
        order: Order,
        pages: CheckoutPages,
        key: string,
    ) => Promise<{ readonly providerId: string; readonly url: string }>;
    /** Expire an open checkout session, or report it complete once its buyer paid. */
    readonly expire: (session: string, key: string) => Promise<"expired" | "complete">;
    /** Replace the prices of a subscription, which its update event then reports. */
    readonly change: (subscription: string, prices: Order["prices"], key: string) => Promise<void>;
    /** Cancel a subscription at once, invoicing its usage so far. */
    readonly cancel: (subscription: string, key: string) => Promise<void>;
    /** Add a priced line to a draft invoice, returning the provider's identifier of the line. */
    readonly bill: (line: InvoiceLine, key: string) => Promise<string>;

    /** Verify a webhook delivery and read its event, absent for an event finance ignores. */
    readonly receive: (request: Request) => Promise<ProviderEvent | undefined>;
}

/** The provider events finance follows, each applied once however often it arrives. */
export const ProviderEvent = {
    /** Read a webhook delivery's event and apply it in finance's space, ignoring other events. */
    async receive(
        home: string,
        provider: PaymentProvider,
        request: Request,
        store: ObjectStore,
    ): Promise<void> {
        const event = await provider.receive(request);
        if (event !== undefined) {
            await ProviderEvent.apply(home, `${provider.name}/${event.id}`, event, store);
        }
    },

    /** Run one step of an event in a transaction as finance, under its keyed request. */
    async step(
        event: {
            readonly home: string;
            readonly key: string;
            readonly time: number;
            readonly store: ObjectStore;
        },
        name: string,
        calls: (reader: Snapshot) => Promise<readonly Call[]>,
    ): Promise<void> {
        // write the calls the step lists, in the transaction its reads ran in
        const { home, key, time, store } = event;
        await Transaction.run(store.database, async (transaction) => {
            const listed = await calls(transaction.view());
            if (listed.length > 0) {
                await store.mutate(
                    store.serving(subscription),
                    { id: await RequestId.derive(time, `${key}/${name}`), calls: [...listed] },
                    { transaction, scope: home },
                );
            }
        });
    },

    /** Apply one event under a redelivery key, each step in a transaction under a keyed request. */
    async apply(
        home: string,
        key: string,
        event: ProviderEvent,
        store: ObjectStore,
    ): Promise<void> {
        // run each step as finance, its reads recorded, under the request a redelivery repeats
        const step = { home, key, time: event.time, store };

        // apply the event by its kind
        if (event.kind === "checkout") {
            await ProviderEvent.step(step, "checkout", (snapshot) =>
                ProviderEvent.checkout(event, snapshot),
            );
        } else if (event.kind === "subscription") {
            await ProviderEvent.step(step, "subscription", (snapshot) =>
                ProviderEvent.subscription(event.subscription, snapshot),
            );
        } else if (event.kind === "invoice") {
            await ProviderEvent.closePeriod(home, key, event, store);
            await ProviderEvent.step(step, "invoice", (snapshot) =>
                ProviderEvent.invoice(home, event.invoice, snapshot),
            );
        } else {
            await ProviderEvent.step(step, "seller", (snapshot) =>
                ProviderEvent.seller(event, snapshot),
            );
        }
    },

    /** Complete an open session with its subscription, or let it expire, unless settled before. */
    async checkout(
        event: Extract<ProviderEvent, { kind: "checkout" }>,
        snapshot: Snapshot,
    ): Promise<Call[]> {
        // read the session
        const [row] = await snapshot
            .read(checkoutSession.table)
            .findMany({ where: { providerId: event.session } });
        if (row === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `no checkout session ${event.session}`,
            });
        }

        // settle an open one with the subscription, expire it, or leave a settled one
        if (row.status !== "open") {
            return [];
        } else if (event.status === "complete") {
            const started = present(event.subscription, "a completed checkout's subscription");

            return [
                checkoutSession.call("settle", {
                    ...checkoutSession.target(row),
                    subscription: started,
                }),
            ];
        } else {
            return [checkoutSession.call("expire", checkoutSession.target(row))];
        }
    },

    /** Follow a subscription finance stores, leaving one a checkout has not started yet. */
    async subscription(provided: ProviderSubscription, snapshot: Snapshot): Promise<Call[]> {
        const [row] = await snapshot
            .read(subscription.table)
            .findMany({ where: { providerId: provided.providerId } });

        return row === undefined
            ? []
            : [subscription.call("sync", { ...subscription.target(row), ...provided })];
    },

    /** Mirror an invoice as the provider has it, creating it once. */
    async invoice(home: string, issued: ProviderInvoice, snapshot: Snapshot): Promise<Call[]> {
        // read the billed account
        const [buyer] = await snapshot
            .read(customer.table)
            .findMany({ where: { providerId: issued.customer } });
        if (buyer === undefined) {
            throw new CallError("NOT_FOUND", { message: `no customer ${issued.customer}` });
        }

        // create the invoice, or update the mirrored one
        const {
            customer: _customer,
            subscription: _subscription,
            isPeriodStart: _start,
            ...fields
        } = issued;
        const [mirrored] = await snapshot
            .read(invoice.table)
            .findMany({ where: { providerId: issued.providerId } });

        return mirrored === undefined
            ? [
                  invoice.call("create", {
                      ...invoice.in(home),
                      accountId: buyer.accountId,
                      ...fields,
                  }),
              ]
            : [invoice.call("update", { ...invoice.target(mirrored), ...fields })];
    },

    /** Close the period a drafted renewal invoice ends, once per period, under a stable request. */
    async closePeriod(
        home: string,
        key: string,
        event: Extract<ProviderEvent, { kind: "invoice" }>,
        store: ObjectStore,
    ): Promise<void> {
        // leave an invoice closing no period, as a first invoice's period is a single instant
        const issued = event.invoice;
        const { periodStart, periodEnd } = issued;
        if (
            issued.status !== "draft" ||
            !issued.isPeriodStart ||
            issued.subscription === null ||
            periodStart === null ||
            periodEnd === null ||
            periodStart >= periodEnd
        ) {
            return;
        }

        // leave a first invoice, whose subscription finance stores once its checkout settles
        const [billed] = await Snapshot.live(store.database)
            .read(subscription.table)
            .findMany({
                where: {
                    providerId: issued.subscription,
                },
            });
        if (billed === undefined) {
            return;
        }

        // close the period onto the invoice
        await store.mutate(
            store.serving(subscription),
            {
                id: await RequestId.derive(event.time, `${key}/close`),
                calls: [
                    subscription.call("closePeriod", {
                        ...subscription.target(billed),
                        periodStart,
                        periodEnd,
                        invoice: { providerId: issued.providerId, customer: issued.customer },
                    }),
                ],
            },
            { scope: home },
        );
    },

    /** Record a seller's capabilities, activating or restricting it as charging starts or stops. */
    async seller(
        event: Extract<ProviderEvent, { kind: "seller" }>,
        snapshot: Snapshot,
    ): Promise<Call[]> {
        // read the seller of the connected account
        const [row] = await snapshot
            .read(seller.table)
            .findMany({ where: { providerId: event.seller } });
        if (row === undefined) {
            throw new CallError("NOT_FOUND", { message: `no seller ${event.seller}` });
        }

        // record its capabilities, and move its status with them
        const { chargesEnabled, payoutsEnabled } = event;
        const recorded = seller.call("record", {
            ...seller.target(row),
            chargesEnabled,
            payoutsEnabled,
        });
        if (chargesEnabled && row.status !== "active") {
            return [recorded, seller.call("activate", seller.target(row))];
        } else if (!chargesEnabled && row.status === "active") {
            return [recorded, seller.call("restrict", seller.target(row))];
        }

        return [recorded];
    },
};

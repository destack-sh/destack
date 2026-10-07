import { eq } from "@destack/db";
import { SystemCall } from "@destack/object";
import type { ObjectServer } from "@destack/object/server";
import { present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import {
    checkoutSession,
    customer,
    type Customer,
    invoice,
    type Invoice,
    type PriceTerms,
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
    /** The provider's identifier of the billed subscription, null for an invoice of no subscription. */
    readonly subscription: string | null;
    /** Whether the invoice starts a billing period, closing the period before it. */
    readonly isPeriodStart: boolean;
};

/** What a provider's webhook delivery tells finance. */
export type ProviderEvent =
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
          /** The invoice as the provider issued it. */
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
      };

/** A purchase of a seller's recurring prices a customer pays at the provider's page. */
export interface Order {
    /** The provider's identifier of the buying customer. */
    readonly customer: string;
    /** The provider's identifier of the seller's connected account, which the charges go to. */
    readonly seller: string;
    /** The provider's identifiers of the licensed prices, with the quantity of each. */
    readonly prices: readonly { readonly price: string; readonly quantity: number }[];
}

/** A priced line added to a customer's invoice, such as a charge for a period's usage or its credit. */
export interface InvoiceLine {
    /** The provider's identifier of the customer. */
    readonly customer: string;
    /** The provider's identifier of the draft invoice the line joins, null for the customer's next invoice. */
    readonly invoice: string | null;
    /** The amount in whole minor units, negative for a credit. */
    readonly amount: number;
    /** The currency of the amount. */
    readonly currency: string;
    /** What the line bills, as the invoice shows it. */
    readonly description: string;
    /** The period the line bills, in UTC epoch milliseconds. */
    readonly period: { readonly start: number; readonly end: number };
    /** The charge the line bills. */
    readonly charge: string;
}

/** A payment provider finance bills through. */
export interface PaymentProvider {
    /** The provider's name, which its webhook path ends with. */
    readonly name: string;

    /** Open a connected account for a seller doing business from a country, returning its identifier. */
    openSeller(country: string, key: string): Promise<string>;
    /** Open the onboarding page of a seller's connected account. */
    onboard(
        seller: string,
        pages: { readonly returnUrl: string; readonly refreshUrl: string },
    ): Promise<string>;
    /** Keep a customer's billing details, returning the provider's identifier. */
    upsertCustomer(customer: Customer, key: string): Promise<string>;
    /** Mirror a new licensed price, and its product once. */
    createPrice(
        product: Product,
        terms: PriceTerms,
        key: string,
    ): Promise<{ readonly product: string; readonly price: string }>;

    /** Open a checkout of an order, returning the provider's identifier of the session and its page. */
    checkout(
        order: Order,
        key: string,
    ): Promise<{ readonly providerId: string; readonly url: string }>;
    /** Replace the prices of a subscription, returning it as the provider bills it then. */
    change(
        subscription: string,
        prices: Order["prices"],
        key: string,
    ): Promise<ProviderSubscription>;
    /** Cancel a subscription at once, invoicing its usage so far. */
    cancel(subscription: string, key: string): Promise<void>;
    /** Add a priced line to a draft invoice, returning the provider's identifier of the line. */
    bill(line: InvoiceLine, key: string): Promise<string>;

    /** Verify a webhook delivery and read what it tells, absent for an event finance does not follow. */
    receive(request: Request): Promise<ProviderEvent | undefined>;
}

/** The provider events finance follows, each applied once. */
export const ProviderEvent = {
    /** Verify a webhook delivery and apply the event it carries, leaving one finance does not follow. */
    async receive(
        server: ObjectServer,
        provider: PaymentProvider,
        request: Request,
    ): Promise<void> {
        const event = await provider.receive(request);
        if (event !== undefined) {
            await ProviderEvent.apply(server, event);
        }
    },

    /** Apply one event a verified webhook delivery carried. */
    async apply(server: ObjectServer, event: ProviderEvent): Promise<void> {
        if (event.kind === "checkout") {
            await ProviderEvent.checkout(server, event);
        } else if (event.kind === "subscription") {
            await ProviderEvent.subscription(server, event.subscription);
        } else if (event.kind === "invoice") {
            await ProviderEvent.invoice(server, event.invoice);
        } else {
            await ProviderEvent.seller(server, event);
        }
    },

    /** Complete an open session with the subscription it started, or let it expire. */
    async checkout(
        server: ObjectServer,
        event: Extract<ProviderEvent, { kind: "checkout" }>,
    ): Promise<void> {
        // read the open session, leaving one settled before
        const [row] = await server.database
            .select()
            .from(checkoutSession.table)
            .where(eq(checkoutSession.table.providerId, event.session));
        if (row === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: `no checkout session ${event.session}`,
            });
        } else if (row.status !== "open") {
            return;
        }

        // settle it with the subscription, or expire it
        const now = server.clock();
        if (event.status === "complete") {
            const started = present(event.subscription, "a completed checkout's subscription");
            await server.executeAsSystem(
                checkoutSession,
                "settle",
                [SystemCall.of(row, { subscription: started })],
                now,
            );
        } else {
            await server.executeAsSystem(checkoutSession, "expire", [SystemCall.of(row)], now);
        }
    },

    /** Follow a subscription finance keeps, leaving one a checkout has not started yet. */
    async subscription(
        server: ObjectServer,
        provided: Extract<ProviderEvent, { kind: "subscription" }>["subscription"],
    ): Promise<void> {
        const [row] = await server.database
            .select()
            .from(subscription.table)
            .where(eq(subscription.table.providerId, provided.providerId));
        if (row === undefined) {
            return;
        }

        await server.executeAsSystem(
            subscription,
            "sync",
            [SystemCall.of(row, provided)],
            server.clock(),
        );
    },

    /** Mirror an invoice, closing the ended period of the subscription a drafted one bills onto it first. */
    async invoice(server: ObjectServer, issued: ProviderInvoice): Promise<void> {
        // read the billed account
        const [buyer] = await server.database
            .select({ scope: customer.table.scope })
            .from(customer.table)
            .where(eq(customer.table.providerId, issued.customer));
        if (buyer === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no customer ${issued.customer}` });
        }

        // close the period a drafted invoice of a renewal ends, adding its charges as lines
        if (issued.status === "draft" && issued.isPeriodStart && issued.subscription !== null) {
            await ProviderEvent.closePeriod(server, issued);
        }

        // create the invoice, or update the one kept
        const {
            customer: _customer,
            subscription: _subscription,
            isPeriodStart: _start,
            ...fields
        } = issued;
        const [kept] = await server.database
            .select()
            .from(invoice.table)
            .where(eq(invoice.table.providerId, issued.providerId));
        const now = server.clock();
        if (kept === undefined) {
            await server.executeAsSystem(
                invoice,
                "create",
                [{ scope: buyer.scope, input: fields }],
                now,
            );
        } else {
            await server.executeAsSystem(invoice, "update", [SystemCall.of(kept, fields)], now);
        }
    },

    /** Close the period a drafted renewal invoice of a subscription ends, once per period. */
    async closePeriod(server: ObjectServer, issued: ProviderInvoice): Promise<void> {
        // read the subscription the invoice bills, leaving a first invoice, which ends no period
        const [billed] = await server.database
            .select()
            .from(subscription.table)
            .where(
                eq(subscription.table.providerId, present(issued.subscription, "a subscription")),
            );
        const { periodStart, periodEnd } = issued;
        if (billed === undefined || periodStart === null || periodEnd === null) {
            return;
        } else if (periodStart >= periodEnd) {
            return;
        }

        // close the period onto the invoice
        await server.executeAsSystem(
            subscription,
            "closePeriod",
            [
                SystemCall.of(billed, {
                    periodStart,
                    periodEnd,
                    invoice: { providerId: issued.providerId, customer: issued.customer },
                }),
            ],
            server.clock(),
        );
    },

    /** Record a seller's capabilities, activating one taking charges and restricting one that stopped. */
    async seller(
        server: ObjectServer,
        event: Extract<ProviderEvent, { kind: "seller" }>,
    ): Promise<void> {
        // read the seller of the connected account
        const [row] = await server.database
            .select()
            .from(seller.table)
            .where(eq(seller.table.providerId, event.seller));
        if (row === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no seller ${event.seller}` });
        }

        // record its capabilities, and move its status with them
        const now = server.clock();
        const { chargesEnabled, payoutsEnabled } = event;
        const [recorded] = await server.executeAsSystem(
            seller,
            "record",
            [SystemCall.of(row, { chargesEnabled, payoutsEnabled })],
            now,
        );
        const current = present(recorded, "the recorded seller");
        if (chargesEnabled && current.status !== "active") {
            await server.executeAsSystem(seller, "activate", [SystemCall.of(current)], now);
        } else if (!chargesEnabled && current.status === "active") {
            await server.executeAsSystem(seller, "restrict", [SystemCall.of(current)], now);
        }
    },
};

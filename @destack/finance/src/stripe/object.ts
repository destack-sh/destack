import { schema } from "@destack/schema";
import type { ProviderSubscription } from "../object/index.ts";
import type { ProviderInvoice } from "../provider/provider.ts";

/** A Stripe time, in seconds since the Unix epoch. */
const Seconds = schema.number().int();

/** A Stripe subscription as API version 2025-09-30 answers it, its periods on its items. */
export const StripeSubscription = schema.looseObject({
    /** The subscription's identifier. */
    id: schema.string(),
    /** The customer's identifier. */
    customer: schema.string(),
    /** Where its billing stands. */
    status: schema.enum([
        "incomplete",
        "incomplete_expired",
        "trialing",
        "active",
        "past_due",
        "canceled",
        "unpaid",
        "paused",
    ]),
    /** Whether it ends with the current period. */
    cancel_at_period_end: schema.boolean(),
    /** When it was canceled. */
    canceled_at: Seconds.nullable(),
    /** When its trial ends. */
    trial_end: Seconds.nullable(),
    /** Its items. */
    items: schema.looseObject({
        /** The items. */
        data: schema.array(
            schema.looseObject({
                /** The item's identifier. */
                id: schema.string(),
                /** The billed price. */
                price: schema.looseObject({ id: schema.string() }),
                /** The licensed quantity, absent on a metered item. */
                quantity: schema.number().int().exactOptional(),
                /** When the item's current period started. */
                current_period_start: Seconds,
                /** When the item's current period ends. */
                current_period_end: Seconds,
            }),
        ),
    }),
});

/** A Stripe invoice as API version 2025-09-30 answers it, its subscription under its parent. */
export const StripeInvoice = schema.looseObject({
    /** The invoice's identifier. */
    id: schema.string(),
    /** The customer's identifier. */
    customer: schema.string(),
    /** The number a finalized invoice carries. */
    number: schema.string().nullable(),
    /** Where it stands. */
    status: schema.enum(["draft", "open", "paid", "void", "uncollectible"]),
    /** The lowercase currency. */
    currency: schema.string(),
    /** The amount before tax. */
    subtotal: schema.number().int(),
    /** The taxes. */
    total_taxes: schema.array(schema.looseObject({ amount: schema.number().int() })).nullable(),
    /** The amount due. */
    total: schema.number().int(),
    /** The amount paid. */
    amount_paid: schema.number().int(),
    /** The start of the billed period. */
    period_start: Seconds,
    /** The end of the billed period. */
    period_end: Seconds,
    /** The hosted page. */
    hosted_invoice_url: schema.string().nullable(),
    /** The PDF. */
    invoice_pdf: schema.string().nullable(),
    /** When it was created. */
    created: Seconds,
    /** Why it was created, such as subscription_cycle. */
    billing_reason: schema.string().nullable(),
    /** What it bills, such as a subscription. */
    parent: schema
        .looseObject({
            /** The billed subscription. */
            subscription_details: schema.looseObject({ subscription: schema.string() }).nullable(),
        })
        .nullable(),
});

/** A Stripe checkout session's outcome. */
export const StripeCheckoutSession = schema.looseObject({
    /** The session's identifier. */
    id: schema.string(),
    /** The subscription a completed session started. */
    subscription: schema.string().nullable(),
});

/** A Stripe connected account's capabilities. */
export const StripeAccount = schema.looseObject({
    /** The account's identifier. */
    id: schema.string(),
    /** Whether it takes charges. */
    charges_enabled: schema.boolean(),
    /** Whether it is paid out. */
    payouts_enabled: schema.boolean(),
});

/** A Stripe webhook event. */
export const StripeEvent = schema.looseObject({
    /** The event's identifier. */
    id: schema.string(),
    /** The event's type, such as invoice.paid. */
    type: schema.string(),
    /** The object it concerns. */
    data: schema.looseObject({ object: schema.record(schema.string(), schema.unknown()) }),
});

/** The billing reasons of invoices starting a period. */
const PERIOD_STARTS: ReadonlySet<string> = new Set(["subscription_create", "subscription_cycle"]);

/** Read Stripe's objects as finance's provider types. */
export const Stripe = {
    /** Read a subscription, taking its period from its first item as all items share it. */
    subscription(input: unknown): ProviderSubscription {
        // require an item, whose period the subscription's is
        const row = StripeSubscription.parse(input);
        const [first] = row.items.data;
        if (first === undefined) {
            throw new TypeError(`stripe subscription ${row.id} has no items`);
        }

        return {
            providerId: row.id,
            customer: row.customer,
            status: row.status,
            currentPeriodStart: first.current_period_start * 1000,
            currentPeriodEnd: first.current_period_end * 1000,
            cancelAtPeriodEnd: row.cancel_at_period_end,
            canceledAt: row.canceled_at === null ? null : row.canceled_at * 1000,
            trialEnd: row.trial_end === null ? null : row.trial_end * 1000,
            items: row.items.data.map((item) => ({
                providerId: item.id,
                price: item.price.id,
                quantity: item.quantity ?? 1,
            })),
        };
    },

    /** Read an invoice. */
    invoice(input: unknown): ProviderInvoice {
        const row = StripeInvoice.parse(input);
        const tax = (row.total_taxes ?? []).reduce((total, each) => total + each.amount, 0);

        return {
            providerId: row.id,
            customer: row.customer,
            subscription: row.parent?.subscription_details?.subscription ?? null,
            isPeriodStart: PERIOD_STARTS.has(row.billing_reason ?? ""),
            number: row.number,
            status: row.status,
            currency: row.currency.toUpperCase(),
            subtotal: row.subtotal,
            tax,
            total: row.total,
            amountPaid: row.amount_paid,
            periodStart: row.period_start * 1000,
            periodEnd: row.period_end * 1000,
            hostedUrl: row.hosted_invoice_url,
            pdfUrl: row.invoice_pdf,
            issuedAt: row.created * 1000,
        };
    },
};

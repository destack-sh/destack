import { account } from "@destack/account/object";
import { check, index, sql, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";
import { BILLING_PERMISSIONS } from "./customer.ts";
import { Currency } from "../rate/amount.ts";
import { seller } from "./seller.ts";

/** A product to subscribe to, and the currency of the recurring prices to subscribe at. */
export const Purchasable = defineSchema(
    schema.object({
        /** The seller's product. */
        product: schema.object({
            /** The seller's account. */
            scope: schema.identifier("account"),
            /** The product. */
            id: schema.identifier("product"),
        }),
        /** The currency of the prices. */
        currency: Currency,
    }),
);
/** A product to subscribe to at its prices in one currency. */
export type Purchasable = schema.Infer<typeof Purchasable>;

/** The states a subscription moves through. */
export const SUBSCRIPTION_STATES = [
    "incomplete",
    "incomplete_expired",
    "trialing",
    "active",
    "past_due",
    "canceled",
    "unpaid",
    "paused",
] as const;

/** A subscription as the payment provider bills it. */
export const ProviderSubscription = defineSchema(
    schema.object({
        /** The provider's identifier of the subscription. */
        providerId: schema.string().min(1),
        /** The provider's identifier of the customer. */
        customer: schema.string().min(1),
        /** Where the provider's billing stands, in finance's states. */
        status: schema.enum(SUBSCRIPTION_STATES),
        /** When the current period started, in UTC epoch milliseconds. */
        currentPeriodStart: schema.number().int(),
        /** When the current period ends. */
        currentPeriodEnd: schema.number().int(),
        /** Whether the subscription ends with the current period. */
        cancelAtPeriodEnd: schema.boolean(),
        /** When the subscription was canceled, null while it runs. */
        canceledAt: schema.number().int().nullable(),
        /** When the trial ends, null without one. */
        trialEnd: schema.number().int().nullable(),
        /** The items, by the provider's identifiers of themselves and their prices. */
        items: schema.array(
            schema.object({
                /** The provider's identifier of the item. */
                providerId: schema.string().min(1),
                /** The provider's identifier of the item's price. */
                price: schema.string().min(1),
                /** The units billed each period. */
                quantity: schema.number().int().min(0),
            }),
        ),
    }),
);
/** A subscription as the payment provider bills it. */
export type ProviderSubscription = schema.Infer<typeof ProviderSubscription>;

/** The lines a period's close added to the provider's invoice. */
export const PeriodClose = defineSchema(
    schema.object({
        /** The provider's identifier of each usage line's invoice line, by charge. */
        lines: schema.record(schema.string(), schema.string()),
        /** The included usage drawn on the period's lines, in minor units. */
        drawn: schema.number().int().min(0),
        /** The provider's identifier of the invoice line crediting what was drawn, null without one. */
        creditLine: schema.string().nullable(),
    }),
);
/** The lines a period's close added to the provider's invoice. */
export type PeriodClose = schema.Infer<typeof PeriodClose>;

/** The work a checkout or plan change did at the provider. */
export const ProviderWork = defineSchema(
    schema.object({
        /** The provider's identifier of the buying customer, once mirrored. */
        customer: schema.string().nullable(),
        /** The checkout the buyer pays at. */
        session: schema.object({ providerId: schema.string(), url: schema.url() }).nullable(),
        /** The subscription as the provider bills it after a change of its prices. */
        subscription: ProviderSubscription.nullable(),
        /** The current period closed onto the final invoice of a subscription canceled for the default product. */
        closed: PeriodClose.nullable(),
    }),
);
/** The external work a checkout or a plan change did at the payment provider. */
export type ProviderWork = schema.Infer<typeof ProviderWork>;

/** The states in which a subscription grants its products' features. */
export const GRANTING_STATES = ["trialing", "active", "past_due"] as const;

/** An account's recurring purchase of a seller's prices, billed each period by the provider. */
export const subscription = defineObject({
    name: "subscription",
    plural: "subscriptions",
    scope: account,
    fields: {
        /** The seller billing the subscription. */
        seller: field.reference(seller, { qualified: true }),
        /** Where the provider's billing of the subscription stands. */
        status: field.state({
            initial: "incomplete",
            transitions: {
                trial: { from: ["incomplete"], to: "trialing", permission: "provide" },
                activate: {
                    from: ["incomplete", "trialing", "past_due", "unpaid", "paused"],
                    to: "active",
                    permission: "provide",
                },
                fail: { from: ["trialing", "active"], to: "past_due", permission: "provide" },
                lapse: { from: ["past_due"], to: "unpaid", permission: "provide" },
                pause: { from: ["trialing", "active"], to: "paused", permission: "provide" },
                expire: { from: ["incomplete"], to: "incomplete_expired", permission: "provide" },
                cancel: {
                    from: ["incomplete", "trialing", "active", "past_due", "unpaid", "paused"],
                    to: "canceled",
                    permission: "provide",
                },
            },
        }),
        /** When the current billing period started, in UTC epoch milliseconds. */
        currentPeriodStart: field.time(),
        /** When the current billing period ends, in UTC epoch milliseconds. */
        currentPeriodEnd: field.time(),
        /** Whether the subscription ends with the current period. */
        cancelAtPeriodEnd: field.boolean().default(false),
        /** When the trial ends. */
        trialEnd: field.time().optional(),
        /** When the subscription was canceled. */
        canceledAt: field.time().optional(),
        /** The most usage may cost past the included usage each period. */
        spendingLimit: field.integer().optional(),
        /** The usage each period includes, its prices' included usage when subscribed, in minor units. */
        includedUsage: field.integer().default(0),
        /** The provider's identifier of the subscription. */
        providerId: field.string(),
    },
    permissions: BILLING_PERMISSIONS,
    reserved: ["provide"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true }),
        update: method.update("bill", { fields: ["cancelAtPeriodEnd", "spendingLimit"] }),
        /** Move the subscription to another product, or cancel it for the seller's default product. */
        change: method.mutation({ permission: "bill", input: Purchasable, prepared: ProviderWork }),
        /** Follow the subscription as the provider bills it: its period, state and items. */
        sync: method.mutation({ permission: null, isSystem: true, input: ProviderSubscription }),
        /** Close an ended period onto the provider's draft invoice. */
        closePeriod: method.mutation({
            permission: null,
            isSystem: true,
            input: schema.object({
                /** When the period started, in UTC epoch milliseconds. */
                periodStart: schema.number().int(),
                /** When it ended. */
                periodEnd: schema.number().int(),
                /** The draft invoice closing the period. */
                invoice: schema.object({
                    /** The provider's identifier of the invoice. */
                    providerId: schema.string().min(1),
                    /** The provider's identifier of the billed customer. */
                    customer: schema.string().min(1),
                }),
            }),
            prepared: PeriodClose,
        }),
        /** Record the provider's period, trial and identifier. */
        record: method.update(null, {
            isSystem: true,
            fields: [
                "currentPeriodStart",
                "currentPeriodEnd",
                "trialEnd",
                "providerId",
                "includedUsage",
            ],
        }),
    }),
    constraints: (entry) => [
        index("subscription_status").on(entry.scope, entry.status),
        check("subscription_period", sql`${entry.currentPeriodStart} < ${entry.currentPeriodEnd}`),
        check(
            "subscription_spending_limit",
            sql`${entry.spendingLimit} IS NULL OR ${entry.spendingLimit} >= 0`,
        ),
    ],
});
/** A persisted subscription. */
export type Subscription = Select<typeof subscription.table>;

import { type Call, ObjectReference } from "@destack/sync";
import { through } from "@destack/access";
import { account, space } from "@destack/account/object";
import { type Select, type Snapshot, Table } from "@destack/db";
import { type MethodContext, type MethodContextOf, defineObject, field } from "@destack/object";
import { defineSchema, type Identifier, present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Currency } from "../rate/amount.ts";
import { BILLING_PERMISSIONS } from "./billing.ts";
import { Price, price, PRICE_TERMS, PriceMirror } from "./price.ts";
import { FeatureGrant } from "../feature/grant.ts";
import { seller } from "./seller.ts";

/** A transition of a subscription's state. */
type Transition = "trial" | "activate" | "fail" | "lapse" | "pause" | "expire" | "cancel";

/** The transitions moving a new subscription into each state. */
const PATHS: Readonly<Record<Subscription["status"], readonly Transition[]>> = {
    incomplete: [],
    incomplete_expired: ["expire"],
    trialing: ["trial"],
    active: ["activate"],
    past_due: ["activate", "fail"],
    unpaid: ["activate", "fail", "lapse"],
    paused: ["activate", "pause"],
    canceled: ["cancel"],
};

/** The transition moving a running subscription into each state, as one provider event tells it. */
const STEPS: Readonly<Record<Subscription["status"], Transition | undefined>> = {
    incomplete: undefined,
    incomplete_expired: "expire",
    trialing: "trial",
    active: "activate",
    past_due: "fail",
    unpaid: "lapse",
    paused: "pause",
    canceled: "cancel",
};

/** The schema of a product to subscribe to, and the currency of the prices to subscribe at. */
const purchasableSchema = defineSchema(
    schema.object({
        /** The seller's product. */
        product: schema.object({
            /** The space of the product. */
            scope: schema.identifier("space"),
            /** The product. */
            id: schema.identifier("product"),
        }),
        /** The currency of the prices. */
        currency: Currency,
    }),
);
/** A product to subscribe to at its prices in one currency. */
export type Purchasable = schema.Infer<typeof purchasableSchema>;

/** A product to subscribe to, and the currency of the recurring prices to subscribe at. */
export const Purchasable = Object.assign(purchasableSchema, {
    /** Read what a checkout buys from its product reference columns and currency. */
    of(input: {
        readonly productScope: string;
        readonly productId: string;
        readonly currency: string;
    }): Purchasable {
        const product = { scope: input.productScope, id: input.productId };

        return Purchasable.parse({ product, currency: input.currency });
    },
});

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
        /** The provider's invoice line per usage charge, null for a line billing nothing. */
        lines: schema.record(schema.string(), schema.string().nullable()),
        /** The included usage drawn on the period's lines, in minor units. */
        drawn: schema.number().int().min(0),
        /** The provider's identifier of the invoice line crediting the drawn usage, if any. */
        creditLine: schema.string().nullable(),
    }),
);
/** The lines a period's close added to the provider's invoice. */
export type PeriodClose = schema.Infer<typeof PeriodClose>;

/** A change of a subscription's plan at the provider. */
export const PlanChange = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** The prices changed. */
            kind: schema.literal("change"),
            /** The provider's identifier of the buying customer. */
            customer: schema.string(),
            /** The provider's identifier of the subscription, changed once the change commits. */
            subscription: schema.string().min(1),
            /** The provider's identifiers of the licensed prices the subscription changes to. */
            prices: schema.array(
                schema.object({ price: schema.string().min(1), quantity: schema.number().int() }),
            ),
            /** The product and prices the change mirrored first. */
            mirrored: PriceMirror,
        }),
        schema.object({
            /** The subscription canceled for the default product. */
            kind: schema.literal("cancel"),
            /** The provider's identifier of the buying customer. */
            customer: schema.string(),
            /** The provider's identifier of the subscription, canceled once the change commits. */
            subscription: schema.string().min(1),
            /** The current period closed onto the final invoice. */
            closed: PeriodClose,
        }),
    ]),
);
/** A change of a subscription's plan at the provider. */
export type PlanChange = schema.Infer<typeof PlanChange>;

/** The states in which a subscription grants its products' features. */
export const GRANTING_STATES = ["trialing", "active", "past_due"] as const;

/** An account's recurring purchase of a seller's prices, billed each period by the provider. */
export const subscription = defineObject(
    {
        name: "subscription",
        plural: "subscriptions",
        scope: space,
        owner: "account",
        fields: {
            /** The account the row belongs to. */
            account: field.reference(account).relate("account"),
            /** The seller billing the subscription. */
            seller: field.object(seller),
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
                    expire: {
                        from: ["incomplete"],
                        to: "incomplete_expired",
                        permission: "provide",
                    },
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
            /** The provider's identifier of the subscription. */
            providerId: field.string(),
        },
        permissions: BILLING_PERMISSIONS,
        reserved: ["provide"],
        constraints: {
            status: { kind: "index", on: ["accountId", "status"] },
            provider: { kind: "index", on: ["providerId"], unique: true },
            granting: {
                kind: "index",
                on: ["accountId", "sellerId"],
                unique: true,
                where: { status: { in: [...GRANTING_STATES] } },
            },
            period: {
                kind: "check",
                condition: { currentPeriodStart: { lt: { field: "currentPeriodEnd" } } },
            },
            spendingLimit: {
                kind: "check",
                condition: {
                    spendingLimit: { OR: [{ isNull: true }, { gte: 0 }] },
                },
            },
        },
    },
    (method) => ({
        create: method.create("provide", { isInternal: true }),
        /** Follow the subscription as the provider bills it: its period, state and items. */
        sync: method.mutation({
            permission: "provide",
            isInternal: true,
            input: ProviderSubscription,
        }),
        /** Move the subscription to another product, or cancel it for the default product. */
        change: method.mutation({ permission: "bill", input: Purchasable, prepared: PlanChange }),
        /** Close an ended period onto the provider's draft invoice. */
        closePeriod: method.mutation({
            permission: "provide",
            isInternal: true,
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
        update: method.update("bill", { fields: ["cancelAtPeriodEnd", "spendingLimit"] }),
    }),
);
/** A persisted subscription. */
export type Subscription = Select<typeof subscription.table>;

/** A price a subscription bills each period, for a quantity. */
export const subscriptionItem = defineObject(
    {
        name: "subscription-item",
        plural: "subscriptionItems",
        scope: space,
        nested: { in: subscription, delete: "cascade", receive: "provide" },
        fields: {
            /** The seller's price. */
            price: field.object(price),
            /** The units billed each period. */
            quantity: field.integer().default(1),
            /** The price's terms at creation. */
            terms: field.json(Table.selectSchema(price.table).pick(PRICE_TERMS)),
            /** The product's feature grants at creation, which entitlements derive from. */
            grants: field.json(schema.array(FeatureGrant)),
            /** The provider's item identifier, absent where no provider bills. */
            providerId: field.string().optional(),
        },
        permissions: {
            read: through("parent", "read"),
            provide: through("parent", "provide"),
        },
        constraints: {
            quantity: { kind: "check", condition: { quantity: { gte: 0 } } },
        },
    },
    (method) => ({
        create: method.create("provide", { isInternal: true }),
        update: method.update("provide", { isInternal: true, fields: ["quantity", "providerId"] }),
        delete: method.delete("provide", { isInternal: true }),
    }),
);
/** A persisted subscription item. */
export type SubscriptionItem = Select<typeof subscriptionItem.table>;

/** The subscriptions accounts have to sellers' prices. */
export const Subscription = {
    /** Read an account's granting subscriptions, each with its items. */
    async granting(
        accountId: Identifier<"account">,
        reader: Pick<Snapshot, "read">,
    ): Promise<(Subscription & { readonly items: readonly SubscriptionItem[] })[]> {
        const rows = await reader.read(subscription.table).findMany({
            where: {
                accountId,
                status: { in: [...GRANTING_STATES] },
            },
        });

        return Subscription.itemized(rows, reader);
    },

    /** Read subscriptions' items, each subscription with its own. */
    async itemized(
        rows: readonly Subscription[],
        reader: Pick<Snapshot, "read">,
    ): Promise<(Subscription & { readonly items: readonly SubscriptionItem[] })[]> {
        // read every subscription's items at once
        if (rows.length === 0) {
            return [];
        }
        const items = await reader.read(subscriptionItem.table).findMany({
            where: {
                parentId: { in: rows.map((row) => row.id) },
            },
        });
        const byParent = Map.groupBy(items, (item) => item.parentId);

        return rows.map((row) => ({ ...row, items: byParent.get(row.id) ?? [] }));
    },

    /** Refuse an account's second granting subscription to a seller, which changes the first. */
    async requireNone(
        accountId: Identifier<"account">,
        sold: { readonly id: string },
        context: Pick<MethodContext, "read">,
    ): Promise<void> {
        const granting = await context
            .read(subscription)
            .findMany({ where: { accountId, status: { in: [...GRANTING_STATES] } } });
        if (granting.some((row) => row.sellerId === sold.id)) {
            throw new CallError("CONFLICT", {
                message: `account ${accountId} subscribes to seller ${sold.id} already, so it changes that subscription`,
            });
        }
    },

    /** Start a subscription as the provider bills it, spending nothing past its included usage. */
    async start(
        sold: Pick<Subscription, "accountId"> & { readonly seller: ObjectReference },
        billed: readonly (Pick<SubscriptionItem, "quantity" | "providerId"> & {
            readonly price: Price;
        })[],
        provided: ProviderSubscription,
        context: MethodContext,
    ): Promise<Subscription> {
        // open it for the provider's period, spending nothing past its included usage
        const includedUsage = SubscriptionItem.includedUsage(
            billed.map((each) => ({ terms: each.price, quantity: each.quantity })),
        );
        const [created] = await context.mutate(context.self, [
            subscription.call("create", {
                accountId: sold.accountId,
                ...ObjectReference.columns("seller", sold.seller),
                currentPeriodStart: provided.currentPeriodStart,
                currentPeriodEnd: provided.currentPeriodEnd,
                cancelAtPeriodEnd: provided.cancelAtPeriodEnd,
                trialEnd: provided.trialEnd,
                canceledAt: provided.canceledAt,
                providerId: provided.providerId,
                spendingLimit: includedUsage > 0 ? 0 : null,
            }),
        ]);

        // bill each price with its terms and grants
        const items = await Promise.all(
            billed.map((each) => SubscriptionItem.create(created.id, each, context)),
        );
        await context.mutate(context.self, items);

        // move it into the provider's state
        let moved: Subscription = created;
        for (const transition of PATHS[provided.status]) {
            moved = await move(moved, transition, context);
        }

        return moved;
    },

    /** Follow a subscription as the provider bills it: its period, its state and its items. */
    async sync(context: MethodContextOf<typeof subscription, "sync">): Promise<Subscription> {
        // record the provider's period, trial and cancellation
        const provided = context.input;
        const recorded = await context.update({
            currentPeriodStart: provided.currentPeriodStart,
            currentPeriodEnd: provided.currentPeriodEnd,
            cancelAtPeriodEnd: provided.cancelAtPeriodEnd,
            trialEnd: provided.trialEnd,
            canceledAt: provided.canceledAt,
        });

        // follow the provider's items
        await syncItems(recorded.id, provided, context);

        // move into the provider's state with the one step it took
        const step = STEPS[provided.status];
        if (provided.status === recorded.status || step === undefined) {
            return recorded;
        }

        return move(recorded, step, context);
    },
};

/** The prices subscriptions bill. */
export const SubscriptionItem = {
    /** Add up the usage items include per period: included usage per unit times quantity. */
    includedUsage(
        items: readonly {
            readonly terms: Pick<Price, "includedUsage">;
            readonly quantity: number;
        }[],
    ): number {
        return items.reduce((total, item) => total + item.terms.includedUsage * item.quantity, 0);
    },

    /** Read a subscription's billing from its items: currency and included usage per period. */
    billing(row: Subscription & { readonly items: readonly SubscriptionItem[] }): {
        readonly currency: string;
        readonly includedUsage: number;
    } {
        const [first] = row.items;

        return {
            currency: present(first, `an item of subscription ${row.id}`).terms.currency,
            includedUsage: SubscriptionItem.includedUsage(row.items),
        };
    },

    /** Read the prices of the provider's items by the identifiers it has for them. */
    async prices(
        items: ProviderSubscription["items"],
        context: Pick<MethodContext, "read">,
    ): Promise<(Pick<SubscriptionItem, "quantity" | "providerId"> & { readonly price: Price })[]> {
        // read the mirrored prices
        if (items.length === 0) {
            return [];
        }
        const rows = await context
            .read(price)
            .findMany({ where: { providerId: { in: items.map((item) => item.price) } } });

        // pair each item with its price, refusing a price finance never mirrored
        return items.map((item) => {
            const found = rows.find((row) => row.providerId === item.price);
            if (found === undefined) {
                throw new CallError("NOT_FOUND", {
                    message: `no price mirrors provider price ${item.price}`,
                });
            }

            return { price: found, quantity: item.quantity, providerId: item.providerId };
        });
    },

    /** Replace a subscription's finance-only metered items with items billing the prices. */
    async replaceMetered(
        subscriptionId: string,
        prices: readonly Price[],
        context: MethodContext,
    ): Promise<void> {
        // delete the stored metered items
        const stored = await context.read(subscriptionItem).findMany({
            where: {
                parentId: subscription.identifier(subscriptionId),
            },
        });
        const deleted = stored
            .filter((each) => Price.isMetered(each.terms))
            .map((row) => subscriptionItem.call("delete", { id: row.id }));

        // add one per price
        const added = await Promise.all(
            prices.map((each) =>
                SubscriptionItem.create(
                    subscriptionId,
                    {
                        price: each,
                        quantity: 1,
                        providerId: null,
                    },
                    context,
                ),
            ),
        );
        await context.mutate(context.self, [...deleted, ...added]);
    },

    /** Build the call creating an item billing a price, with the price's terms and grants. */
    async create(
        subscriptionId: string,
        billed: Pick<SubscriptionItem, "quantity" | "providerId"> & { readonly price: Price },
        reader: Pick<Snapshot, "read">,
    ): Promise<Call> {
        const reference = { scope: billed.price.scope, id: billed.price.id };
        const offered = await Price.offer(reference, reader);

        return subscriptionItem.call("create", {
            parentId: subscriptionId,
            ...ObjectReference.columns("price", price.reference(reference.scope, reference.id)),
            quantity: billed.quantity,
            providerId: billed.providerId,
            ...offered,
        });
    },
};

/** Replace a subscription's items with the provider's, leaving the ones it still bills. */
async function syncItems(
    subscriptionId: string,
    provided: ProviderSubscription,
    context: MethodContext,
): Promise<void> {
    // delete the items the provider dropped, and follow the quantities of the others
    const stored = await context.read(subscriptionItem).findMany({
        where: {
            parentId: subscription.identifier(subscriptionId),
        },
    });
    const wanted = new Map(provided.items.map((item) => [item.providerId, item]));
    const changed = stored.flatMap((row): Call[] => {
        const item = row.providerId === null ? null : wanted.get(row.providerId);
        if (item === undefined) {
            return [subscriptionItem.call("delete", { id: row.id })];
        } else if (item !== null && item.quantity !== row.quantity) {
            return [subscriptionItem.call("update", { id: row.id, quantity: item.quantity })];
        }

        return [];
    });

    // add the items it added, by the prices it bills
    const mirrored = new Set(stored.map((row) => row.providerId));
    const added = provided.items.filter((item) => !mirrored.has(item.providerId));
    const created = await Promise.all(
        (await SubscriptionItem.prices(added, context)).map((each) =>
            SubscriptionItem.create(subscriptionId, each, context),
        ),
    );
    await context.mutate(context.self, [...changed, ...created]);
}

/** Move a subscription through one transition as finance. */
async function move(
    row: Subscription,
    transition: Transition,
    context: MethodContext,
): Promise<Subscription> {
    const [moved] = await context.mutate(context.self, [
        subscription.call(transition, { ...subscription.target(row) }),
    ]);

    return moved;
}

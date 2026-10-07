import { type DatabaseConnection, eq, inArray } from "@destack/db";
import type { Call, CallOf } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import {
    Price,
    price,
    type ProviderSubscription,
    subscription,
    type Subscription,
    type SubscriptionItem,
    subscriptionItem,
} from "../object/index.ts";

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

/** Start a subscription to a seller's prices as the provider bills it, spending nothing past included usage unless it replaces one. */
export async function startSubscription(
    call: Call,
    sold: Pick<Subscription, "seller"> & Partial<Pick<Subscription, "spendingLimit">>,
    billed: readonly (Pick<SubscriptionItem, "quantity" | "providerId"> & {
        readonly price: Price;
    })[],
    provided: ProviderSubscription,
): Promise<Subscription> {
    // open it for the provider's period
    const includedUsage = billed.reduce(
        (total, each) => total + each.price.includedUsage * each.quantity,
        0,
    );
    const created = await call.invoke(subscription).create({
        seller: sold.seller,
        currentPeriodStart: provided.currentPeriodStart,
        currentPeriodEnd: provided.currentPeriodEnd,
        cancelAtPeriodEnd: provided.cancelAtPeriodEnd,
        trialEnd: provided.trialEnd,
        providerId: provided.providerId,
        includedUsage,
        spendingLimit:
            sold.spendingLimit === undefined ? (includedUsage > 0 ? 0 : null) : sold.spendingLimit,
    });

    // bill each price, keeping its terms and grants
    for (const each of billed) {
        await addItem(call, created.id, each);
    }

    // move it into the provider's state
    let moved: Subscription = created;
    for (const transition of PATHS[provided.status]) {
        moved = await move(call, moved, transition);
    }

    return moved;
}

/** Follow a subscription as the provider bills it: its period, its state and its items. */
export async function syncSubscription(
    call: CallOf<typeof subscription, "sync">,
): Promise<Subscription> {
    // record the provider's period and trial
    const provided = call.input;
    const current = call.target;
    await call.update({
        currentPeriodStart: provided.currentPeriodStart,
        currentPeriodEnd: provided.currentPeriodEnd,
        cancelAtPeriodEnd: provided.cancelAtPeriodEnd,
        trialEnd: provided.trialEnd,
    });

    // move into the provider's state with the one step it took
    const step = STEPS[provided.status];
    const moved =
        provided.status === current.status || step === undefined
            ? current
            : await move(call, current, step);

    // replace the items the provider no longer bills
    await syncItems(call, current.id, provided);

    return moved;
}

/** Replace a subscription's items with the provider's, keeping the ones it still bills. */
async function syncItems(
    call: Call,
    subscriptionId: string,
    provided: ProviderSubscription,
): Promise<void> {
    // delete the items the provider dropped
    const kept = await call.database
        .select({
            id: subscriptionItem.table.id,
            providerId: subscriptionItem.table.providerId,
        })
        .from(subscriptionItem.table)
        .where(eq(subscriptionItem.table.parentId, subscription.identifier(subscriptionId)));
    const wanted = new Set(provided.items.map((item) => item.providerId));
    for (const row of kept.filter(
        (each) => each.providerId !== null && !wanted.has(each.providerId),
    )) {
        await call.invoke(subscriptionItem).delete({ id: row.id });
    }

    // add the items it added, by the prices it bills
    const mirrored = new Set(kept.map((row) => row.providerId));
    const added = provided.items.filter((item) => !mirrored.has(item.providerId));
    for (const each of await readItemPrices(call.database, added)) {
        await addItem(call, subscriptionId, each);
    }
}

/** Read the prices of the provider's items by the identifiers it keeps for them. */
export async function readItemPrices(
    database: DatabaseConnection,
    items: ProviderSubscription["items"],
): Promise<(Pick<SubscriptionItem, "quantity" | "providerId"> & { readonly price: Price })[]> {
    // read the mirrored prices
    if (items.length === 0) {
        return [];
    }
    const rows = await database
        .select()
        .from(price.table)
        .where(
            inArray(
                price.table.providerId,
                items.map((item) => item.price),
            ),
        );

    // pair each item with its price, refusing a price finance never mirrored
    return items.map((item) => {
        const found = rows.find((row) => row.providerId === item.price);
        if (found === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: `no price mirrors provider price ${item.price}`,
            });
        }

        return { price: found, quantity: item.quantity, providerId: item.providerId };
    });
}

/** Replace a subscription's metered items, which only finance keeps, with items billing the prices. */
export async function replaceMetered(
    call: Call,
    subscriptionId: string,
    prices: readonly Price[],
): Promise<void> {
    // delete the metered items kept
    const kept = await call.database
        .select({ id: subscriptionItem.table.id, terms: subscriptionItem.table.terms })
        .from(subscriptionItem.table)
        .where(eq(subscriptionItem.table.parentId, subscription.identifier(subscriptionId)));
    for (const row of kept.filter((each) => each.terms.recurring?.usage === "metered")) {
        await call.invoke(subscriptionItem).delete({ id: row.id });
    }

    // add one per price
    for (const each of prices) {
        await addItem(call, subscriptionId, { price: each, quantity: 1, providerId: null });
    }
}

/** Add an item billing a price to a subscription, keeping the price's terms and grants. */
async function addItem(
    call: Call,
    subscriptionId: string,
    billed: Pick<SubscriptionItem, "quantity" | "providerId"> & { readonly price: Price },
): Promise<void> {
    const reference = { scope: billed.price.scope, id: billed.price.id };
    const offered = await Price.offer(call.database, reference);
    await call.invoke(subscriptionItem).create({
        parentId: subscriptionId,
        price: reference,
        quantity: billed.quantity,
        providerId: billed.providerId,
        ...offered,
    });
}

/** Move a subscription through one transition as the system. */
async function move(call: Call, row: Subscription, transition: Transition): Promise<Subscription> {
    return call.invoke(subscription)[transition]({ id: row.id });
}

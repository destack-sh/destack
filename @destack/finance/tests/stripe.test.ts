import { asc, eq } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { present } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    charge,
    checkoutSession,
    Entitlement,
    invoice,
    price,
    product,
    productFeature,
    seller,
    subscription,
    subscriptionItem,
} from "../src/object/index.ts";
import { ProviderEvent } from "../src/provider/index.ts";
import { StripeProvider } from "../src/stripe/index.ts";
import { type StripeDelivery, StripeFixture } from "../src/test/index.ts";
import { Finance, ids } from "./fixture/finance.ts";
import { calls, disk, diskSku, requests, storage } from "./fixture/storage.ts";

/** The webhook endpoint finance serves Stripe's deliveries at. */
const WEBHOOK = "https://finance.test/finance/webhooks/stripe";

/** The monthly fee of the Go plan, and the usage it includes, in euro cents. */
const FEE = 900;

/** A gigabyte, as storage prices count it. */
const GB = 1_000_000_000;

/** A monthly recurrence. */
const MONTHLY = { interval: "month", intervalCount: 1 } as const;

/** Open finance billing through the Stripe fixture, with Carol's seller onboarded and Acme's customer. */
async function open(dialect: (typeof TEST_DIALECTS)[number]) {
    // bill through the fixture, reading its keys as the vault would hand them
    const stripe = new StripeFixture(Date.now());
    const provider = new StripeProvider({
        key: stripe.key,
        webhookSecret: stripe.webhookSecret,
        fetch: stripe.fetch,
        pages: {
            successUrl: "https://destack.test/billing",
            cancelUrl: "https://destack.test/plans",
        },
        now: () => stripe.now * 1000,
    });
    const finance = await Finance.open(dialect, { provider });
    onTestFinished(() => finance.close());

    // open Carol's connected account, finish its onboarding, and keep Acme's customer
    const offer = await finance.offer();
    const deliver = (deliveries: readonly StripeDelivery[]) =>
        stripe.deliver(
            deliveries,
            async (request) => {
                await ProviderEvent.receive(finance.server, provider, request);

                return new Response(null, { status: 204 });
            },
            WEBHOOK,
        );
    const [shop] = await finance.test.database.select().from(seller.table);
    await deliver(
        stripe.onboard(present(present(shop, "shop").providerId, "a provider identifier")),
    );
    await finance.customer();

    return { stripe, provider, finance, offer, deliver };
}

/** Offer a plan in Carol's shop at some prices granting unlimited calls, including some usage in its fee when given, returning the product. */
async function offerPlan(
    finance: Finance,
    name: string,
    prices: readonly Record<string, unknown>[],
    includedUsage = 0,
) {
    // create the product granting calls
    const plan = await finance.call("carol", product, "create", ids.shop, {
        name,
        description: `The ${name} plan.`,
    });
    await finance.call("carol", productFeature, "create", ids.shop, {
        parentId: plan.id,
        packageId: storage.id,
        feature: calls.name,
        value: null,
    });

    // price it, the licensed fee including the usage
    for (const terms of prices) {
        const isFee = terms["unitAmount"] !== undefined;
        await finance.call("carol", price, "create", ids.shop, {
            parentId: plan.id,
            ...terms,
            ...(isFee ? { includedUsage } : {}),
        });
    }

    return { scope: ids.shop, id: plan.id };
}

/** Offer the Go plan: 9 euro including 9 euro of usage, requests at 10 cents each and disk at its list price. */
function offerGo(finance: Finance) {
    return offerPlan(
        finance,
        "Go",
        [
            {
                currency: "EUR",
                unitAmount: FEE,
                recurring: { ...MONTHLY, usage: "licensed" },
            },
            {
                currency: "EUR",
                unitAmountDecimal: "10",
                recurring: { ...MONTHLY, usage: "metered", meter: requests.reference },
            },
        ],
        FEE,
    );
}

/** Offer the free plan as Carol's default product, granting unlimited calls without prices. */
async function offerFree(finance: Finance) {
    const plan = await finance.call("carol", product, "create", ids.shop, {
        name: "Free",
        description: "The Free plan.",
        isDefault: true,
    });
    await finance.call("carol", productFeature, "create", ids.shop, {
        parentId: plan.id,
        packageId: storage.id,
        feature: calls.name,
        value: null,
    });

    return { scope: ids.shop, id: plan.id };
}

/** Read the buyer's subscriptions with their items' provider identifiers, oldest first. */
async function subscriptions(finance: Finance) {
    const rows = await finance.test.database
        .select()
        .from(subscription.table)
        .where(eq(subscription.table.scope, ids.buyer))
        .orderBy(asc(subscription.table.createdAt));
    const items = await finance.test.database.select().from(subscriptionItem.table);

    return rows.map((row) => ({
        status: row.status,
        isBilled: row.providerId !== null,
        items: items
            .filter((item) => item.parentId === row.id)
            .map((item) => [item.terms.recurring?.usage, item.providerId !== null]),
    }));
}

test.each(TEST_DIALECTS)(
    "subscribe through Checkout, start the subscription from the signed webhook and mirror the invoice on %s",
    async (dialect) => {
        // open a checkout of the Go plan as Alice
        const { stripe, finance, deliver } = await open(dialect);
        const go = await offerGo(finance);
        const session = await finance.call("alice", checkoutSession, "create", ids.buyer, {
            product: go,
            currency: "EUR",
        });

        // pay at the page, and take Stripe's deliveries
        const statuses = await deliver(
            stripe.complete(present(session.providerId, "the session's provider identifier")),
        );
        await finance.entitle();
        const [settled] = await finance.test.database
            .select({
                status: checkoutSession.table.status,
                subscriptionId: checkoutSession.table.subscriptionId,
            })
            .from(checkoutSession.table);
        const [mirrored] = await finance.test.database
            .select({
                status: invoice.table.status,
                total: invoice.table.total,
                currency: invoice.table.currency,
            })
            .from(invoice.table);
        const entitled = Entitlement.resolve(
            await finance.entitlements(ids.buyer),
            calls.reference,
        );

        expect({
            url: session.url,
            statuses,
            session: { status: settled?.status, isLinked: settled?.subscriptionId !== null },
            subscriptions: await subscriptions(finance),
            invoice: mirrored,
            entitled:
                entitled?.kind === "metered"
                    ? [entitled.limit, entitled.usage, entitled.state]
                    : null,
        }).toEqual({
            url: `https://checkout.stripe.test/pay/${present(session.providerId, "the session's provider identifier")}`,
            statuses: [204, 204],
            session: { status: "complete", isLinked: true },
            subscriptions: [
                {
                    status: "active",
                    isBilled: true,
                    items: [
                        ["licensed", true],
                        ["metered", false],
                    ],
                },
            ],
            invoice: { status: "paid", total: FEE, currency: "EUR" },
            entitled: [null, 0, "within"],
        });
    },
);

test.each(TEST_DIALECTS)(
    "start a plan including usage at no spending past it, and a pay-as-you-go plan without a spending limit on %s",
    async (dialect) => {
        // subscribe through Checkout to Go and to a plan billing requests past a fee of nothing
        const { stripe, finance, deliver } = await open(dialect);
        const metered = await offerPlan(finance, "Metered", [
            {
                currency: "EUR",
                unitAmount: 0,
                recurring: { ...MONTHLY, usage: "licensed" },
            },
            {
                currency: "EUR",
                unitAmountDecimal: "10",
                recurring: { ...MONTHLY, usage: "metered", meter: requests.reference },
            },
        ]);
        for (const plan of [await offerGo(finance), metered]) {
            const session = await finance.call("alice", checkoutSession, "create", ids.buyer, {
                product: plan,
                currency: "EUR",
            });
            await deliver(
                stripe.complete(present(session.providerId, "the session's provider identifier")),
            );
        }

        // keep Go's spending at its included usage, and leave the other's unlimited
        const rows = await finance.test.database
            .select({
                includedUsage: subscription.table.includedUsage,
                spendingLimit: subscription.table.spendingLimit,
            })
            .from(subscription.table)
            .where(eq(subscription.table.scope, ids.buyer))
            .orderBy(asc(subscription.table.createdAt));
        expect(rows).toEqual([
            { includedUsage: FEE, spendingLimit: 0 },
            { includedUsage: 0, spendingLimit: null },
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "rate a seller's metered price into one line a repeated use counts once, near the included usage, and invoice it less the included usage on %s",
    async (dialect) => {
        // subscribe to Go through Checkout
        const { stripe, finance, deliver } = await open(dialect);
        const go = await offerGo(finance);
        const session = await finance.call("alice", checkoutSession, "create", ids.buyer, {
            product: go,
            currency: "EUR",
        });
        await deliver(
            stripe.complete(present(session.providerId, "the session's provider identifier")),
        );
        await finance.entitle();

        // route 72 requests as a cell would, twice
        const requested = {
            id: "1",
            source: "/spaces/a",
            meter: { packageId: storage.id, name: requests.name },
            quantity: 72,
            time: stripe.now * 1000,
        };
        await finance.fixture.use(ids.buyer, [requested]);
        await finance.fixture.use(ids.buyer, [requested]);
        const entitled = Entitlement.resolve(
            await finance.entitlements(ids.buyer),
            calls.reference,
        );

        // renew at Stripe: the draft closing the period takes the line and the included usage drawn on it
        const [subscribed] = stripe.list("sub").map((each) => String(each["id"]));
        const renewal = stripe.renew(present(subscribed, "the stripe subscription"));
        await deliver(renewal);
        const drafted = String(present(renewal[0], "the drafted invoice").object["id"]);

        // keep 7.20 euro near the 9 euro included, and invoice it drawn on the included usage
        expect({
            state: entitled?.kind === "metered" ? entitled.state : null,
            lines: await finance.test.database
                .select({
                    category: charge.table.chargeCategory,
                    service: charge.table.serviceName,
                    quantity: charge.table.pricingQuantity,
                    listCost: charge.table.listCost,
                    billedCost: charge.table.billedCost,
                })
                .from(charge.table)
                .orderBy(asc(charge.table.chargeCategory)),
            items: stripe.list("ii").map((item) => [item["invoice"], item["amount"]]),
        }).toEqual({
            state: "near",
            lines: [
                {
                    category: "Credit",
                    service: null,
                    quantity: null,
                    listCost: "0",
                    billedCost: "-720",
                },
                {
                    category: "Usage",
                    service: "Go",
                    quantity: 72,
                    listCost: "720",
                    billedCost: "720",
                },
            ],
            items: [
                [drafted, "720"],
                [drafted, "-720"],
            ],
        });
    },
);

test.each(TEST_DIALECTS)(
    "refuse a webhook delivery whose payload changed after Stripe signed it on %s",
    async (dialect) => {
        // open a checkout of the Go plan
        const { stripe, provider, finance } = await open(dialect);
        const go = await offerGo(finance);
        const session = await finance.call("alice", checkoutSession, "create", ids.buyer, {
            product: go,
            currency: "EUR",
        });

        // deliver its completion, its payload changed on the way
        const [completed] = stripe.complete(
            present(session.providerId, "the session's provider identifier"),
        );
        const failure = await refusal(
            stripe.deliver(
                [present(completed, "completed")],
                async (request) => {
                    const changed = (await request.text()).replace("complete", "completed");
                    await ProviderEvent.receive(
                        finance.server,
                        provider,
                        new Request(request.url, {
                            method: "POST",
                            headers: request.headers,
                            body: changed,
                        }),
                    );

                    return new Response(null, { status: 204 });
                },
                WEBHOOK,
            ),
        );

        // keep the session open
        const [kept] = await finance.test.database
            .select({ status: checkoutSession.table.status })
            .from(checkoutSession.table);
        expect({ failure, kept }).toEqual({
            failure: ["UNAUTHORIZED", "forged stripe signature"],
            kept: { status: "open" },
        });
    },
);

test.each(TEST_DIALECTS)(
    "move from the default product to Go through Checkout, and back by closing the period onto Stripe's final invoice and canceling on %s",
    async (dialect) => {
        // derive the default product's calls, then subscribe to Go through Checkout
        const { stripe, finance, deliver } = await open(dialect);
        const free = await offerFree(finance);
        const go = await offerGo(finance);
        await finance.entitle();
        const source = async () =>
            (await finance.entitlements(ids.buyer)).map((row) => row.source.type);
        const onFree = await source();
        const session = await finance.call("alice", checkoutSession, "create", ids.buyer, {
            product: go,
            currency: "EUR",
        });
        await deliver(
            stripe.complete(present(session.providerId, "the session's provider identifier")),
        );
        await finance.entitle();
        const onGo = await source();

        // route 72 requests, then change back to the default product
        await finance.fixture.use(ids.buyer, [
            {
                id: "1",
                source: "/spaces/a",
                meter: { packageId: storage.id, name: requests.name },
                quantity: 72,
                time: stripe.now * 1000,
            },
        ]);
        const [held] = await finance.test.database.select().from(subscription.table);
        await finance.call("alice", subscription, "change", ids.buyer, {
            id: present(held, "the Go subscription").id,
            product: free,
            currency: "EUR",
        });
        const [canceled] = stripe.list("sub").map((each) => String(each["id"]));
        await deliver(stripe.canceled(present(canceled, "the canceled Stripe subscription")));
        await finance.entitle();

        // bill the requests and the included usage drawn on them to the next invoice, cancel, and fall back to the default
        expect({
            onFree,
            onGo,
            items: stripe.list("ii").map((item) => [item["invoice"] ?? null, item["amount"]]),
            subscriptions: (await subscriptions(finance)).map((each) => each.status),
            atStripe: stripe.list("sub").map((each) => each["status"]),
            onFreeAgain: await source(),
        }).toEqual({
            onFree: ["product"],
            onGo: ["subscription-item", "subscription-item"],
            items: [
                [null, "720"],
                [null, "-720"],
            ],
            subscriptions: ["canceled"],
            atStripe: ["canceled"],
            onFreeAgain: ["product"],
        });
    },
);

test.each(
    TEST_DIALECTS.flatMap((dialect) => [
        { dialect, limit: "the default spending limit", spendingLimit: 0, billed: 900 },
        { dialect, limit: "no spending limit", spendingLimit: null, billed: 1000 },
    ]),
)(
    "invoice a period's rated usage on the renewal's draft invoice, less the included usage, forgiving usage past $limit on $dialect",
    async ({ dialect, spendingLimit, billed }) => {
        // subscribe to Go through Checkout
        const { stripe, finance, deliver } = await open(dialect);
        const go = await offerGo(finance);
        const session = await finance.call("alice", checkoutSession, "create", ids.buyer, {
            product: go,
            currency: "EUR",
        });
        await deliver(
            stripe.complete(present(session.providerId, "the session's provider identifier")),
        );

        // keep the subscription's spending limit, or lift it
        const [started] = await finance.test.database
            .select({ id: subscription.table.id, spendingLimit: subscription.table.spendingLimit })
            .from(subscription.table)
            .where(eq(subscription.table.scope, ids.buyer));
        const held = present(started, "the started subscription");
        if (spendingLimit !== held.spendingLimit) {
            await finance.call("alice", subscription, "update", ids.buyer, {
                id: held.id,
                spendingLimit,
            });
        }

        // keep 500 GB-months of disk in a space, 10 euro at 2 cents a GB-month
        const now = stripe.now * 1000;
        await finance.fixture.use(ids.buyer, [
            {
                id: "1",
                source: "/spaces/a",
                meter: { packageId: storage.id, name: disk.name },
                quantity: 500 * GB,
                level: 500 * GB,
                sku: diskSku.reference,
                time: now,
            },
        ]);

        // renew at Stripe: the draft closing the period takes the lines, then pay it
        const [subscribed] = stripe.list("sub").map((each) => String(each["id"]));
        const renewal = stripe.renew(present(subscribed, "the stripe subscription"));
        await deliver(renewal);
        const drafted = String(present(renewal[0], "the drafted invoice").object["id"]);
        await deliver([{ type: "invoice.paid", object: stripe.pay(drafted) }]);
        const [customerId] = stripe.list("cus").map((each) => each["id"]);
        const lines = await finance.test.database
            .select({
                category: charge.table.chargeCategory,
                listCost: charge.table.listCost,
                billedCost: charge.table.billedCost,
                status: charge.table.status,
            })
            .from(charge.table)
            .orderBy(asc(charge.table.chargeCategory));
        const [mirrored] = await finance.test.database
            .select({ total: invoice.table.total })
            .from(invoice.table)
            .where(eq(invoice.table.providerId, drafted));
        // bill the usage up to the limit on the draft, drawing the 9 euro prepaid: the next fee and the rest due, all of it earned
        expect({
            items: stripe
                .list("ii")
                .map((item) => [item["customer"], item["invoice"], item["amount"]]),
            lines,
            total: mirrored?.total,
        }).toEqual({
            items: [
                [customerId, drafted, String(billed)],
                [customerId, drafted, "-900"],
            ],
            lines: [
                { category: "Credit", listCost: "0", billedCost: "-900", status: "invoiced" },
                {
                    category: "Usage",
                    listCost: "1000",
                    billedCost: String(billed),
                    status: "invoiced",
                },
            ],
            total: FEE + billed - FEE,
        });
    },
);

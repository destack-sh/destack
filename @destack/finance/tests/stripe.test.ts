import { type Message, t } from "@destack/locale";
import { asc, eq, Snapshot } from "@destack/db";
import { JsonValue, present, schema } from "@destack/schema";
import { RequestId, ObjectReference } from "@destack/sync";
import { subjectContext, TEST_START } from "@destack/object/test";
import type { CallInput } from "@destack/object";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    charge,
    checkoutSession,
    customer,
    invoice,
    price,
    product,
    productFeature,
    seller,
    subscription,
    SubscriptionItem,
    subscriptionItem,
} from "../src/object/index.ts";
import { ProviderEvent } from "../src/provider/index.ts";
import { Stripe, StripeProvider } from "../src/provider/stripe/index.ts";
import { spaces, type StripeDelivery, StripeFixture } from "../src/test/index.ts";
import { Finance, ids, resolved } from "./fixture/finance.ts";
import { Period } from "../src/rate/period.ts";
import { calls, disk, diskSku, requests, storage } from "./fixture/storage.ts";

/** The endpoint finance takes Stripe's deliveries at, below the machine's webhook mount. */
const WEBHOOK = "https://finance--home--destack.destack.space/.destack/webhook/stripe";

/** The pages Checkout returns buyers to. */
const PAGES = {
    successUrl: "https://destack.test/billing",
    cancelUrl: "https://destack.test/plans",
};

/** The monthly fee of the Go plan, and the usage it includes, in euro cents. */
const FEE = 900;

/** A gigabyte, as storage prices count it. */
const GB = 1_000_000_000;

/** The period the Stripe fixture bills a subscription item for, 30 days in seconds. */
const STRIPE_MONTH_SECONDS = 30 * 86_400;

/** A monthly recurrence. */
const MONTHLY = { interval: "month", intervalCount: 1 } as const;

/** The parts of a Stripe subscription the tests read: its items and their prices. */
const StripeSubscription = schema.looseObject({
    items: schema.looseObject({
        data: schema.array(
            schema.looseObject({
                price: schema.looseObject({ unit_amount: schema.string() }),
            }),
        ),
    }),
});

/** Open finance billing through the Stripe fixture, Carol's seller onboarded. */
async function open() {
    // bill through the fixture, reading its keys as the vault would hand them
    const stripe = new StripeFixture(TEST_START);
    const finance = await Finance.open({
        provider: (home) => new StripeProvider({ key: stripe.key, home, fetch: stripe.fetch }),
    });
    const provider = present(finance.fixture.provider, "the Stripe provider");
    onTestFinished(() => finance.close());

    // open Carol's connected account at her onboarding, finish it, and create Acme's customer
    const offer = await finance.offer();
    await finance.call(
        "carol",
        seller.call("onboard", {
            id: offer.seller.id,
            country: "AT",
            returnUrl: "https://destack.test/selling",
            refreshUrl: "https://destack.test/selling/again",
        }),
    );
    const deliver = (deliveries: readonly StripeDelivery[]) =>
        stripe.deliver(
            deliveries,
            async (request) => {
                await ProviderEvent.receive(finance.fixture.home, provider, request, finance.store);

                return new Response(null, { status: 204 });
            },
            WEBHOOK,
        );
    const [shop] = await finance.database.select().from(seller.table);
    await deliver(
        stripe.onboard(present(present(shop, "shop").providerId, "a provider identifier")),
    );
    await finance.customer();

    return { stripe, provider, finance, offer, deliver };
}

/** Offer a plan in Carol's shop granting unlimited calls, returning the product. */
async function offerPlan(
    finance: Finance,
    name: Message,
    prices: readonly Omit<CallInput<typeof price, "create">, "parentId">[],
    includedUsage = 0,
) {
    // create the product granting calls
    const plan = await finance.call(
        "carol",
        product.call("create", {
            accountId: ids.shop,
            name,
            description: t`A plan.`,
        }),
    );
    await finance.call(
        "carol",
        productFeature.call("create", {
            parentId: plan.id,
            packageId: storage.id,
            feature: calls.name,
            value: null,
        }),
    );

    // price it, the licensed fee including the usage
    for (const terms of prices) {
        const isFee = terms.unitAmount !== undefined;
        await finance.call(
            "carol",
            price.call("create", {
                parentId: plan.id,
                ...terms,
                ...(isFee ? { includedUsage } : {}),
            }),
        );
    }

    return { scope: finance.fixture.home, id: plan.id };
}

/** Offer Go: 9 euro including 9 euro of usage, requests at 10 cents, disk at list price. */
function offerGo(finance: Finance) {
    return offerPlan(
        finance,
        t`Go`,
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

/** Offer Pro: 18 euro including 18 euro of usage, without metered prices. */
function offerPro(finance: Finance) {
    return offerPlan(
        finance,
        t`Pro`,
        [{ currency: "EUR", unitAmount: 2 * FEE, recurring: { ...MONTHLY, usage: "licensed" } }],
        2 * FEE,
    );
}

/** Offer the free plan as Carol's default product, granting unlimited calls without prices. */
async function offerFree(finance: Finance) {
    const plan = await finance.call(
        "carol",
        product.call("create", {
            accountId: ids.shop,
            name: t`Free`,
            description: t`The Free plan.`,
            isDefault: true,
        }),
    );
    await finance.call(
        "carol",
        productFeature.call("create", {
            parentId: plan.id,
            packageId: storage.id,
            feature: calls.name,
            value: null,
        }),
    );

    return { scope: finance.fixture.home, id: plan.id };
}

/** Read the buyer's subscriptions with their items' provider identifiers, oldest first. */
async function subscriptions(finance: Finance) {
    const rows = await finance.database
        .select()
        .from(subscription.table)
        .where(eq(subscription.table.accountId, ids.buyer))
        .orderBy(asc(subscription.table.createdAt));
    const items = await finance.database.select().from(subscriptionItem.table);

    return rows.map((row) => ({
        status: row.status,
        isBilled: row.providerId !== null,
        items: items
            .filter((item) => item.parentId === row.id)
            .map((item) => [item.terms.recurring.usage, item.providerId !== null]),
    }));
}

test("open a seller's connected account in the country it onboards from, refusing another country", async () => {
    // onboard Carol's shop from Austria at opening, then try again from Switzerland
    const { finance, offer } = await open();
    const [shop] = await finance.database
        .select({ providerId: seller.table.providerId, country: seller.table.country })
        .from(seller.table);
    const again = await refusal(
        finance.call(
            "carol",
            seller.call("onboard", {
                id: offer.seller.id,
                country: "CH",
                returnUrl: "https://destack.test/selling",
                refreshUrl: "https://destack.test/selling/again",
            }),
        ),
    );

    // expect the connected account in Austria, and the Swiss onboarding refused
    expect([
        present(shop, "shop").providerId === null,
        present(shop, "shop").country,
        again,
    ]).toEqual([
        false,
        "AT",
        [
            "PRECONDITION_FAILED",
            "COUNTRY_IMMUTABLE",
            `seller ${offer.seller.id} is already registered in AT`,
        ],
    ]);
});

test("subscribe through Checkout, start the subscription from the signed webhook and mirror the invoice", async () => {
    // open a checkout of the Go plan as Alice
    const { stripe, finance, deliver } = await open();
    const go = await offerGo(finance);
    const session = await finance.call(
        "alice",
        checkoutSession.call("create", {
            ...PAGES,
            accountId: ids.buyer,
            ...ObjectReference.columns("product", product.reference(go.scope, go.id)),
            currency: "EUR",
        }),
    );

    // pay at the page, and take Stripe's deliveries
    const statuses = await deliver(
        stripe.complete(present(session.providerId, "the session's provider identifier")),
    );
    await finance.entitle();
    const [settled] = await finance.database
        .select({
            status: checkoutSession.table.status,
            subscriptionId: checkoutSession.table.subscriptionId,
        })
        .from(checkoutSession.table);
    const [mirrored] = await finance.database
        .select({
            status: invoice.table.status,
            total: invoice.table.total,
            currency: invoice.table.currency,
        })
        .from(invoice.table);
    const entitled = resolved(await finance.resolutions(ids.buyer), calls.reference);

    expect({
        url: session.url,
        statuses,
        session: { status: settled?.status, isLinked: settled?.subscriptionId !== null },
        subscriptions: await subscriptions(finance),
        invoice: mirrored,
        entitled:
            entitled?.kind === "metered" ? [entitled.limit, entitled.usage, entitled.state] : null,
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
});

test("replace an abandoned checkout by expiring it at Stripe, and refuse another once the buyer paid the open one", async () => {
    // open a checkout of Go twice, as a buyer coming back after closing the page
    const { stripe, finance, deliver } = await open();
    const go = await offerGo(finance);
    const checkout = () =>
        finance.call(
            "alice",
            checkoutSession.call("create", {
                ...PAGES,
                accountId: ids.buyer,
                ...ObjectReference.columns("product", product.reference(go.scope, go.id)),
                currency: "EUR",
            }),
        );
    const abandoned = await checkout();
    const current = await checkout();

    // pay the current one, and refuse a third before Stripe reports the payment
    const paid = stripe.complete(present(current.providerId, "the current session"));
    const refused = await refusal(checkout());
    await deliver(paid);
    const sessions = await finance.database
        .select({ id: checkoutSession.table.id, status: checkoutSession.table.status })
        .from(checkoutSession.table);

    expect({
        sessions: Object.fromEntries(sessions.map((row) => [row.id, row.status])),
        atStripe: stripe.status(present(abandoned.providerId, "the abandoned session")),
        refused,
        subscriptions: (await subscriptions(finance)).map((row) => row.status),
    }).toEqual({
        sessions: { [abandoned.id]: "expired", [current.id]: "complete" },
        atStripe: "expired",
        refused: [
            "CONFLICT",
            "CHECKOUT_COMPLETE",
            `checkout session ${current.id} is paid, and its subscription starts once the provider reports it`,
        ],
        subscriptions: ["active"],
    });
});

test("start a plan including usage at no spending past it, and a pay-as-you-go plan without a spending limit", async () => {
    // subscribe Acme's buyer to Go and Dave's account to a plan billing requests past a fee of nothing, both through Checkout
    const { stripe, finance, deliver } = await open();
    const metered = await offerPlan(finance, t`Metered`, [
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
    await finance.call(
        "dave",
        customer.call("create", {
            accountId: finance.platform,
            email: "billing@destack.test",
            name: "Destack",
        }),
    );
    const purchases = [
        ["alice", ids.buyer, await offerGo(finance)],
        ["dave", finance.platform, metered],
    ] as const;
    for (const [person, accountId, plan] of purchases) {
        const session = await finance.call(
            person,
            checkoutSession.call("create", {
                ...PAGES,
                accountId,
                ...ObjectReference.columns("product", product.reference(plan.scope, plan.id)),
                currency: "EUR",
            }),
        );
        await deliver(
            stripe.complete(present(session.providerId, "the session's provider identifier")),
        );
    }

    // cap Go's spending at its included usage, and leave the other's unlimited
    const rows = await finance.database
        .select()
        .from(subscription.table)
        .orderBy(asc(subscription.table.createdAt));
    const snapshot = Snapshot.live(finance.database);
    const terms = await Promise.all(
        rows.map(async (row) => ({
            includedUsage: SubscriptionItem.includedUsage(
                await snapshot
                    .read(subscriptionItem.table)
                    .findMany({ where: { parentId: row.id } }),
            ),
            spendingLimit: row.spendingLimit,
        })),
    );
    expect(terms).toEqual([
        { includedUsage: FEE, spendingLimit: 0 },
        { includedUsage: 0, spendingLimit: null },
    ]);
});

test("rate a seller's metered price into one line a repeated use counts once, near the included usage, and invoice it less the included usage", async () => {
    // subscribe to Go through Checkout
    const { stripe, finance, deliver } = await open();
    const go = await offerGo(finance);
    const session = await finance.call(
        "alice",
        checkoutSession.call("create", {
            ...PAGES,
            accountId: ids.buyer,
            ...ObjectReference.columns("product", product.reference(go.scope, go.id)),
            currency: "EUR",
        }),
    );
    await deliver(
        stripe.complete(present(session.providerId, "the session's provider identifier")),
    );
    await finance.entitle();

    // route 72 requests as a cloud machine would, twice
    const requested = {
        id: "1",
        space: spaces.notes,
        meter: { packageId: storage.id, name: requests.name },
        quantity: 72,
        time: stripe.now * 1000,
    };
    await finance.fixture.use(ids.buyer, [requested]);
    await finance.fixture.use(ids.buyer, [requested]);
    const entitled = resolved(await finance.resolutions(ids.buyer), calls.reference);

    // renew at Stripe: the draft closing the period takes the line and the included usage drawn on it
    const [subscribed] = stripe.list("sub").map((each) => String(each["id"]));
    const renewal = stripe.renew(present(subscribed, "the stripe subscription"));
    await deliver(renewal);
    const drafted = String(present(renewal[0], "the drafted invoice").object["id"]);

    // spend 7.20 euro near the 9 euro included, and invoice it drawn on the included usage
    expect({
        state: entitled?.kind === "metered" ? entitled.state : null,
        lines: await finance.database
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
});

test("apply only the event Stripe has, refusing an unknown event, and leave another home's events", async () => {
    // open a checkout of the Go plan
    const { stripe, provider, finance } = await open();
    const go = await offerGo(finance);
    const session = await finance.call(
        "alice",
        checkoutSession.call("create", {
            ...PAGES,
            accountId: ids.buyer,
            ...ObjectReference.columns("product", product.reference(go.scope, go.id)),
            currency: "EUR",
        }),
    );
    const post = (body: unknown) =>
        ProviderEvent.receive(
            finance.fixture.home,
            provider,
            new Request(WEBHOOK, { method: "POST", body: JSON.stringify(body) }),
            finance.store,
        );

    // refuse an event Stripe never sent
    const forged = await refusal(post({ id: "evt_forged", type: "checkout.session.completed" }));

    // complete the session from the event Stripe has, whatever the delivery claims
    const [completed] = stripe.complete(present(session.providerId, "the session"));
    await stripe.deliver(
        [present(completed, "completed")],
        async (request) => {
            const delivered = JsonValue.object(await request.json());
            await post({ ...delivered, type: "checkout.session.expired" });

            return new Response(null, { status: 204 });
        },
        WEBHOOK,
    );

    // leave an invoice of another home's customer
    stripe.objects.set("cus_elsewhere", {
        id: "cus_elsewhere",
        object: "customer",
        metadata: { destack_home: "space-elsewhere" },
    });
    const statuses = await stripe.deliver(
        [
            {
                type: "invoice.created",
                object: { id: "in_elsewhere", object: "invoice", customer: "cus_elsewhere" },
            },
        ],
        async (request) => {
            await post(await request.json());

            return new Response(null, { status: 204 });
        },
        WEBHOOK,
    );
    const [settled] = await finance.database
        .select({ status: checkoutSession.table.status })
        .from(checkoutSession.table);
    const invoices = await finance.database
        .select({ providerId: invoice.table.providerId })
        .from(invoice.table);

    expect({
        forged,
        settled,
        statuses,
        invoices: invoices.map((row) => row.providerId),
    }).toEqual({
        forged: [
            "NOT_FOUND",
            "STRIPE_NOT_FOUND",
            "stripe has no /v1/events/evt_forged: No such object: 'evt_forged'",
        ],
        settled: { status: "complete" },
        statuses: [204],
        invoices: [],
    });
});

test("move from the default product to Go through Checkout, and back by closing the period onto Stripe's final invoice and canceling", async () => {
    // reset the default product's calls with the calendar month, then Go's with its Stripe period
    const { stripe, finance, deliver } = await open();
    const free = await offerFree(finance);
    const go = await offerGo(finance);
    await finance.entitle();
    const resetAt = async () => {
        const row = resolved(await finance.resolutions(ids.buyer), calls.reference);

        return row?.kind === "metered" ? row.resetAt : null;
    };
    const onFree = await resetAt();
    const session = await finance.call(
        "alice",
        checkoutSession.call("create", {
            ...PAGES,
            accountId: ids.buyer,
            ...ObjectReference.columns("product", product.reference(go.scope, go.id)),
            currency: "EUR",
        }),
    );
    await deliver(
        stripe.complete(present(session.providerId, "the session's provider identifier")),
    );
    await finance.entitle();
    const onGo = await resetAt();

    // route 72 requests, then change back to the default product
    await finance.fixture.use(ids.buyer, [
        {
            id: "1",
            space: spaces.notes,
            meter: { packageId: storage.id, name: requests.name },
            quantity: 72,
            time: stripe.now * 1000,
        },
    ]);
    const [held] = await finance.database.select().from(subscription.table);
    await finance.call(
        "alice",
        subscription.call("change", {
            id: present(held, "the Go subscription").id,
            product: free,
            currency: "EUR",
        }),
    );
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
        onFreeAgain: await resetAt(),
    }).toEqual({
        onFree: Period.month(finance.fixture.clock.now()).end,
        onGo: (stripe.now + STRIPE_MONTH_SECONDS) * 1000,
        items: [
            [null, "720"],
            [null, "-720"],
        ],
        subscriptions: ["canceled"],
        atStripe: ["canceled"],
        onFreeAgain: Period.month(finance.fixture.clock.now()).end,
    });
});

test("change Go to Pro at Stripe once the change commits, leaving Stripe as it was when a change rolls back", async () => {
    // subscribe to Go through Checkout, with Pro on offer beside it
    const { stripe, finance, deliver } = await open();
    const go = await offerGo(finance);
    const pro = await offerPro(finance);
    const session = await finance.call(
        "alice",
        checkoutSession.call("create", {
            ...PAGES,
            accountId: ids.buyer,
            ...ObjectReference.columns("product", product.reference(go.scope, go.id)),
            currency: "EUR",
        }),
    );
    await deliver(
        stripe.complete(present(session.providerId, "the session's provider identifier")),
    );
    const [held] = await finance.database.select().from(subscription.table);
    const id = present(held, "the Go subscription").id;
    const fees = () =>
        stripe
            .list("sub")
            .flatMap((each) =>
                StripeSubscription.parse(each).items.data.map((item) => item.price.unit_amount),
            );

    // roll back a change to Pro whose mutation also changes to a product not on sale
    const change = (wanted: { readonly scope: string; readonly id: string }) =>
        subscription.call("change", {
            ...subscription.in(finance.fixture.home),
            id,
            product: wanted,
            currency: "EUR",
        });
    const missing = {
        scope: finance.fixture.home,
        id: "product-019f5530-8000-7000-8000-0000000000ff",
    };
    const rolledBack = await refusal(
        finance.store.mutate(
            subjectContext(finance.fixture.subject("alice"), finance.fixture.home, {
                environment: finance.fixture.environment,
            }).access(),
            {
                id: RequestId.create(finance.fixture.environment),
                calls: [change(pro), change(missing)],
            },
        ),
    );
    const afterRollback = fees();

    // change to Pro, then take Stripe's update of the items
    await finance.call("alice", subscription.call("change", { id, product: pro, currency: "EUR" }));
    const afterCommit = fees();
    const [changed] = stripe.list("sub").map((each) => String(each["id"]));
    await deliver(stripe.updated(present(changed, "the changed Stripe subscription")));

    // leave Go's fee at Stripe after the rollback, then bill Pro's alone, its metered prices gone
    expect({
        rolledBack,
        afterRollback,
        afterCommit,
        subscriptions: await subscriptions(finance),
    }).toEqual({
        rolledBack: [
            "NOT_FOUND",
            "PRODUCT_UNAVAILABLE",
            "product product-019f5530-8000-7000-8000-0000000000ff is not on sale",
        ],
        afterRollback: [String(FEE)],
        afterCommit: [String(2 * FEE)],
        subscriptions: [{ status: "active", isBilled: true, items: [["licensed", true]] }],
    });
});

test.each([
    { limit: "the default spending limit", spendingLimit: 0, billed: 900 },
    { limit: "no spending limit", spendingLimit: null, billed: 1000 },
])(
    "invoice a period's rated usage on the renewal's draft invoice, less the included usage, forgiving usage past $limit",
    async ({ spendingLimit, billed }) => {
        // subscribe to Go through Checkout
        const { stripe, finance, deliver } = await open();
        const go = await offerGo(finance);
        const session = await finance.call(
            "alice",
            checkoutSession.call("create", {
                ...PAGES,
                accountId: ids.buyer,
                ...ObjectReference.columns("product", product.reference(go.scope, go.id)),
                currency: "EUR",
            }),
        );
        await deliver(
            stripe.complete(present(session.providerId, "the session's provider identifier")),
        );

        // leave the subscription's spending limit, or lift it
        const [started] = await finance.database
            .select({ id: subscription.table.id, spendingLimit: subscription.table.spendingLimit })
            .from(subscription.table)
            .where(eq(subscription.table.accountId, ids.buyer));
        const held = present(started, "the started subscription");
        if (spendingLimit !== held.spendingLimit) {
            await finance.call(
                "alice",
                subscription.call("update", {
                    id: held.id,
                    spendingLimit,
                }),
            );
        }

        // store 500 GB-months of disk in a space, 10 euro at 2 cents a GB-month
        const now = stripe.now * 1000;
        await finance.fixture.use(ids.buyer, [
            {
                id: "1",
                space: spaces.notes,
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
        const lines = await finance.database
            .select({
                category: charge.table.chargeCategory,
                listCost: charge.table.listCost,
                billedCost: charge.table.billedCost,
                status: charge.table.status,
            })
            .from(charge.table)
            .orderBy(asc(charge.table.chargeCategory));
        const [mirrored] = await finance.database
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

test("apply an invoice event once however often and however concurrently Stripe delivers it, and mirror Stripe's current invoice whatever order its events arrive in", async () => {
    // complete a checkout of Go, holding back the paid invoice's event
    const { stripe, finance, deliver } = await open();
    const go = await offerGo(finance);
    const session = await finance.call(
        "alice",
        checkoutSession.call("create", {
            ...PAGES,
            accountId: ids.buyer,
            ...ObjectReference.columns("product", product.reference(go.scope, go.id)),
            currency: "EUR",
        }),
    );
    const [completed, paid] = stripe.complete(
        present(session.providerId, "the session's provider identifier"),
    );
    await deliver([present(completed, "the completed checkout")]);

    // apply the paid invoice's event twice at once, then once more
    const event = {
        id: "evt_paid",
        time: stripe.now * 1000,
        kind: "invoice",
        invoice: Stripe.invoice(present(paid, "the paid invoice").object),
    } as const;
    const apply = () =>
        ProviderEvent.apply(finance.fixture.home, "stripe/evt_paid", event, finance.store);
    await Promise.all([apply(), apply()]);
    await apply();

    // deliver the invoice's draft again after it was paid
    const draft = { ...present(paid, "the paid invoice").object, status: "draft" };
    await deliver([{ type: "invoice.created", object: draft }]);

    // expect one invoice, paid as Stripe has it now
    const mirrored = await finance.database
        .select({ status: invoice.table.status })
        .from(invoice.table);
    expect(mirrored).toEqual([{ status: "paid" }]);
});

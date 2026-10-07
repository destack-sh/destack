import { StripeSignature } from "../stripe/signature.ts";
import type { StripeSecret } from "../stripe/client.ts";

/** The API key the fixture accepts. */
const API_KEY = "sk_test_destack";

/** The webhook signing secret the fixture signs deliveries with. */
const WEBHOOK_SECRET = "whsec_destack";

/** A month in seconds, the fixture's billing period. */
const MONTH_SECONDS = 30 * 86_400;

/** A Stripe object as the fixture keeps it. */
type StripeObject = Record<string, unknown>;

/** An event the fixture delivers to the webhook endpoint. */
export interface StripeDelivery {
    /** The event type, such as invoice.paid. */
    readonly type: string;
    /** The object the event carries. */
    readonly object: StripeObject;
}

/** A route of the fixture's API: its method, its path pattern and its handler. */
interface Route {
    /** The HTTP method. */
    readonly method: string;
    /** The path pattern, its identifier captured. */
    readonly pattern: RegExp;
    /** Answer a request's parameters, with the captured identifier. */
    readonly handle: (parameters: StripeObject, id: string) => StripeObject;
}

/** An in-process Stripe serving the API subset finance uses, after stripe-mock. */
export class StripeFixture {
    /** The API key, as the vault would hand it. */
    readonly key: StripeSecret = secretOf(API_KEY);
    /** The webhook signing secret, as the vault would hand it. */
    readonly webhookSecret: StripeSecret = secretOf(WEBHOOK_SECRET);
    /** The kept objects, by identifier. */
    readonly objects = new Map<string, StripeObject>();
    /** The answers kept per idempotency key. */
    readonly #answers = new Map<string, StripeObject>();
    /** The count of objects created, which numbers identifiers. */
    #created = 0;
    /** The current time, in seconds. */
    now: number;
    /** The routes of the API. */
    readonly #routes: readonly Route[];

    /** Start the fixture at a time in UTC epoch milliseconds. */
    constructor(now: number) {
        this.now = Math.floor(now / 1000);
        this.#routes = [
            this.#route("POST", /^\/v1\/accounts$/u, (input) =>
                this.#create("acct", { ...input, charges_enabled: false, payouts_enabled: false }),
            ),
            this.#route("POST", /^\/v1\/account_links$/u, (input) => ({
                object: "account_link",
                url: `https://connect.stripe.test/setup/${String(input["account"])}`,
            })),
            this.#route("POST", /^\/v1\/customers$/u, (input) => this.#create("cus", input)),
            this.#route("POST", /^\/v1\/customers\/([^/]+)$/u, (input, id) =>
                this.#update(id, input),
            ),
            this.#route("POST", /^\/v1\/products$/u, (input) => this.#create("prod", input)),
            this.#route("POST", /^\/v1\/prices$/u, (input) => this.#create("price", input)),
            this.#route("POST", /^\/v1\/invoiceitems$/u, (input) =>
                this.#create("ii", { object: "invoiceitem", ...input }),
            ),
            this.#route("POST", /^\/v1\/checkout\/sessions$/u, (input) => this.#checkout(input)),
            this.#route("GET", /^\/v1\/subscriptions\/([^/]+)$/u, (_input, id) => this.#object(id)),
            this.#route("POST", /^\/v1\/subscriptions\/([^/]+)$/u, (input, id) =>
                this.#change(id, input),
            ),
            this.#route("DELETE", /^\/v1\/subscriptions\/([^/]+)$/u, (_input, id) =>
                this.#update(id, { status: "canceled", canceled_at: this.now }),
            ),
        ];
    }

    /** Answer an API request as Stripe would, keeping an answer per idempotency key. */
    readonly fetch = async (request: Request): Promise<Response> => {
        // refuse a request without the key
        if (request.headers.get("authorization") !== `Bearer ${API_KEY}`) {
            return Response.json({ error: { message: "invalid api key" } }, { status: 401 });
        }

        // answer a repeated key as before
        const key = request.headers.get("idempotency-key");
        const kept = key === null ? undefined : this.#answers.get(key);
        if (kept !== undefined) {
            return Response.json(kept);
        }

        // route the request with its form parameters
        const url = new URL(request.url);
        const form = request.method === "GET" ? url.search.slice(1) : await request.text();
        const parameters = decodeForm(form);
        for (const route of this.#routes) {
            const matched =
                route.method === request.method ? route.pattern.exec(url.pathname) : null;
            if (matched !== null) {
                const answer = route.handle(parameters, matched[1] ?? "");
                if (key !== null) {
                    this.#answers.set(key, answer);
                }

                return Response.json(answer);
            }
        }

        return Response.json(
            { error: { message: `no route ${request.method} ${url.pathname}` } },
            { status: 404 },
        );
    };

    /** Tell of a subscription canceled at its customer's request. */
    canceled(subscriptionId: string): StripeDelivery[] {
        return [{ type: "customer.subscription.deleted", object: this.#object(subscriptionId) }];
    }

    /** Complete a Checkout session as its buyer pays. */
    complete(sessionId: string): StripeDelivery[] {
        // start the subscription with the session's prices for a month
        const session = this.#object(sessionId);
        const lines = asObjects(session["line_items"]);
        const subscription = this.#create("sub", {
            object: "subscription",
            customer: session["customer"],
            status: "active",
            cancel_at_period_end: false,
            canceled_at: null,
            trial_end: null,
            items: { data: [] },
        });
        const items = lines.map((line) => this.#item(String(subscription["id"]), line));
        const started = this.#update(String(subscription["id"]), { items: { data: items } });

        // complete the session and invoice the period
        const completed = this.#update(sessionId, {
            status: "complete",
            subscription: started["id"],
        });

        // invoice the first period, its start alone, paid
        const first = this.#invoice(started, "subscription_create");

        return [
            { type: "checkout.session.completed", object: completed },
            { type: "invoice.paid", object: this.pay(String(first["id"])) },
        ];
    }

    /** Start a subscription's next period: draft the invoice closing the ended one, then move the period on. */
    renew(subscriptionId: string): StripeDelivery[] {
        // draft the invoice of the ended period
        const subscription = this.#object(subscriptionId);
        const drafted = this.#invoice(subscription, "subscription_cycle");

        // move each item's period a month on
        const items = asObjects(asObject(subscription["items"])["data"]).map((item) => ({
            ...item,
            current_period_start: item["current_period_end"],
            current_period_end: Number(item["current_period_end"]) + MONTH_SECONDS,
        }));
        const renewed = this.#update(subscriptionId, { items: { data: items } });

        return [
            { type: "invoice.created", object: drafted },
            { type: "customer.subscription.updated", object: renewed },
        ];
    }

    /** Finalize and pay a draft invoice with the invoice items added to it. */
    pay(invoiceId: string): StripeObject {
        // add up the invoice's own amount and its items
        const drafted = this.#object(invoiceId);
        const lines = this.#all("ii").filter((item) => item["invoice"] === invoiceId);
        const total =
            Number(drafted["subtotal"]) +
            lines.reduce((sum, line) => sum + Number(line["amount"]), 0);

        return this.#update(invoiceId, {
            status: "paid",
            subtotal: total,
            total,
            amount_paid: total,
        });
    }

    /** Finish a connected account's onboarding, enabling charges and payouts. */
    onboard(accountId: string): StripeDelivery[] {
        const account = this.#update(accountId, { charges_enabled: true, payouts_enabled: true });

        return [{ type: "account.updated", object: account }];
    }

    /** Deliver events to the webhook endpoint as signed posts, returning each answer's status. */
    async deliver(
        deliveries: readonly StripeDelivery[],
        endpoint: (request: Request) => Promise<Response>,
        url: string,
    ): Promise<number[]> {
        const statuses: number[] = [];
        for (const delivery of deliveries) {
            // sign the event's payload as Stripe signs a delivery
            const payload = JSON.stringify({
                id: `evt_${++this.#created}`,
                object: "event",
                type: delivery.type,
                created: this.now,
                data: { object: delivery.object },
            });
            const signature = await StripeSignature.sign(payload, WEBHOOK_SECRET, this.now * 1000);
            const response = await endpoint(
                new Request(url, {
                    method: "POST",
                    headers: { "content-type": "application/json", "stripe-signature": signature },
                    body: payload,
                }),
            );
            statuses.push(response.status);
        }

        return statuses;
    }

    /** List the kept objects of a kind by their identifier's prefix, oldest first. */
    list(prefix: string): StripeObject[] {
        return this.#all(prefix);
    }

    /** Declare a route. */
    #route(method: string, pattern: RegExp, handle: Route["handle"]): Route {
        return { method, pattern, handle };
    }

    /** Create an object under a new identifier of a prefix. */
    #create(prefix: string, input: StripeObject): StripeObject {
        // number the identifier and keep the object
        const id = `${prefix}_${String(++this.#created).padStart(6, "0")}`;
        const created = { id, created: this.now, ...input };
        this.objects.set(id, created);

        return created;
    }

    /** Merge parameters into a kept object. */
    #update(id: string, input: StripeObject): StripeObject {
        const updated = { ...this.#object(id), ...input };
        this.objects.set(id, updated);

        return updated;
    }

    /** Read a kept object, refusing an unknown one. */
    #object(id: string): StripeObject {
        const found = this.objects.get(id);
        if (found === undefined) {
            throw new TypeError(`the stripe fixture keeps no ${id}`);
        }

        return found;
    }

    /** List the kept objects of a prefix. */
    #all(prefix: string): StripeObject[] {
        return [...this.objects.values()].filter((each) =>
            String(each["id"]).startsWith(`${prefix}_`),
        );
    }

    /** Open a Checkout session paying at its page. */
    #checkout(input: StripeObject): StripeObject {
        const created = this.#create("cs", {
            object: "checkout.session",
            status: "open",
            subscription: null,
            ...input,
        });
        const id = String(created["id"]);

        return this.#update(id, { url: `https://checkout.stripe.test/pay/${id}` });
    }

    /** Replace a subscription's items: delete the ones marked deleted and add the new prices. */
    #change(id: string, input: StripeObject): StripeObject {
        // read the items, the deleted ones and the new prices
        const current = asObjects(asObject(this.#object(id)["items"])["data"]);
        const changes = asObjects(input["items"]);
        const deleted = new Set(
            changes.filter((each) => each["deleted"] === "true").map((each) => each["id"]),
        );
        const added = changes
            .filter((each) => each["price"] !== undefined)
            .map((line) => this.#item(id, line));

        return this.#update(id, {
            items: { data: [...current.filter((item) => !deleted.has(item["id"])), ...added] },
        });
    }

    /** Create a subscription item of a line, for a month from now. */
    #item(subscriptionId: string, line: StripeObject): StripeObject {
        const quantity =
            line["quantity"] === undefined ? {} : { quantity: Number(line["quantity"]) };

        return this.#create("si", {
            object: "subscription_item",
            subscription: subscriptionId,
            price: this.#object(String(line["price"])),
            ...quantity,
            current_period_start: this.now,
            current_period_end: this.now + MONTH_SECONDS,
        });
    }

    /** Draft an invoice of a subscription's licensed prices for its current period. */
    #invoice(subscription: StripeObject, reason: string): StripeObject {
        // charge the licensed prices of the period
        const items = asObjects(asObject(subscription["items"])["data"]);
        const licensed = items.filter(
            (item) => asObject(asObject(item["price"])["recurring"])["usage_type"] === "licensed",
        );
        const total = licensed.reduce(
            (sum, item) =>
                sum +
                Number(asObject(item["price"])["unit_amount"] ?? 0) * Number(item["quantity"] ?? 1),
            0,
        );
        const first = asObject(items[0]);
        const price = asObject(first["price"]);

        return this.#create("in", {
            object: "invoice",
            customer: subscription["customer"],
            number: `DS-${String(this.#created).padStart(4, "0")}`,
            status: "draft",
            currency: price["currency"],
            subtotal: total,
            total_taxes: [],
            total,
            amount_paid: 0,
            period_start:
                reason === "subscription_create" ? this.now : first["current_period_start"],
            period_end: reason === "subscription_create" ? this.now : first["current_period_end"],
            hosted_invoice_url: "https://invoice.stripe.test/i",
            invoice_pdf: "https://invoice.stripe.test/i.pdf",
            billing_reason: reason,
            parent: { subscription_details: { subscription: subscription["id"] } },
        });
    }
}

/** Hand a text secret as the vault reads it. */
function secretOf(value: string): StripeSecret {
    return { read: async () => ({ value: { encoding: "text", value } }) };
}

/** Decode a Stripe form into nested objects, keeping arrays as index-keyed objects listed by `asObjects`. */
export function decodeForm(form: string): StripeObject {
    const decoded: StripeObject = {};
    for (const [name, value] of new URLSearchParams(form)) {
        // walk the bracketed path, creating nested objects
        const path = name.replaceAll("]", "").split("[");
        let target = decoded;
        for (const key of path.slice(0, -1)) {
            const next = target[key];
            const nested: StripeObject = isObject(next) ? next : {};
            target[key] = nested;
            target = nested;
        }
        target[path.at(-1) ?? name] = value;
    }

    return decoded;
}

/** Decide whether a value is an object the fixture keeps. */
function isObject(value: unknown): value is StripeObject {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Read a value as an object. */
function asObject(value: unknown): StripeObject {
    if (typeof value !== "object" || value === null) {
        throw new TypeError("the stripe fixture expected an object");
    }

    return { ...value };
}

/** Read an array, or an index-keyed object a form decoded, as a list of objects. */
function asObjects(value: unknown): StripeObject[] {
    if (value === undefined) {
        return [];
    }

    return Object.values(Array.isArray(value) ? value : asObject(value)).map(asObject);
}

import { expect, test } from "@destack/test";
import { StripeFixture } from "../src/test/stripe.ts";
import { StripeClient } from "../src/provider/stripe/client.ts";
import { StripeEndpoint } from "../src/provider/stripe/endpoint.ts";
import { STRIPE_EVENTS } from "../src/provider/stripe/provider.ts";

/** The URL finance's endpoint in the eu home receives Stripe's events at. */
const URL = "https://finance--home--destack.destack.space/.destack/webhook/stripe";

test("upsert one enabled endpoint at a URL delivering exactly finance's events, however often upserted", async () => {
    // upsert the endpoint twice
    const stripe = new StripeFixture(Date.UTC(2026, 9, 12));
    const client = new StripeClient(stripe.key, stripe.fetch);
    const created = await StripeEndpoint.upsert(client, URL);
    const again = await StripeEndpoint.upsert(client, URL);

    // restore it once someone disabled it and changed its events at Stripe
    await client.request("POST", `/v1/webhook_endpoints/${created}`, {
        enabled_events: ["invoice.paid"],
        disabled: true,
    });
    const restored = await StripeEndpoint.upsert(client, URL);
    const endpoints = stripe.list("we").map((endpoint) => ({
        id: endpoint["id"],
        url: endpoint["url"],
        status: endpoint["status"],
        events: endpoint["enabled_events"],
    }));

    expect({ again, restored, endpoints }).toEqual({
        again: created,
        restored: created,
        endpoints: [
            { id: created, url: URL, status: "enabled", events: [...STRIPE_EVENTS].toSorted() },
        ],
    });
});

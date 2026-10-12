import { schema } from "@destack/schema";
import { type StripeClient, STRIPE_VERSION } from "./client.ts";
import { StripeObject } from "./object.ts";
import { STRIPE_EVENTS } from "./provider.ts";

/** The most endpoints one listing returns, above the 16 Stripe allows an account. */
const ENDPOINT_PAGE = 100;

/** A Stripe webhook endpoint: where it delivers, which events, and whether it delivers. */
const StripeEndpointObject = schema.looseObject({
    /** The endpoint's identifier. */
    id: schema.string(),
    /** The URL it delivers to. */
    url: schema.string(),
    /** The event types it delivers. */
    enabled_events: schema.array(schema.string()),
    /** Whether it delivers. */
    status: schema.enum(["enabled", "disabled"]),
});

/** The webhook endpoints Stripe delivers finance's events to. */
export const StripeEndpoint = {
    /** Keep one enabled endpoint at a URL delivering finance's events, returning its identifier. */
    async keep(client: StripeClient, url: string): Promise<string> {
        // find the endpoint at the URL among the account's
        const listed = await client.request("GET", "/v1/webhook_endpoints", {
            limit: ENDPOINT_PAGE,
        });
        const endpoints = schema.array(StripeEndpointObject).parse(listed["data"]);
        const kept = endpoints.find((endpoint) => endpoint.url === url);

        // create a missing endpoint at the API version finance reads
        const events = [...STRIPE_EVENTS].toSorted();
        if (kept === undefined) {
            const created = await client.request("POST", "/v1/webhook_endpoints", {
                url,
                enabled_events: events,
                api_version: STRIPE_VERSION,
            });

            return StripeObject.parse(created).id;
        }

        // enable a kept one and subscribe it to exactly finance's events
        const isCurrent =
            kept.status === "enabled" &&
            kept.enabled_events.toSorted().join(" ") === events.join(" ");
        if (!isCurrent) {
            await client.request("POST", `/v1/webhook_endpoints/${kept.id}`, {
                enabled_events: events,
                disabled: false,
            });
        }

        return kept.id;
    },
};

import { refusal } from "@destack/object";
import type { AdapterFetch } from "@destack/object/server";
import type { ServerContext } from "@destack/object/workload";
import { SecretReading } from "@destack/resource";
import type { Webhook, WebhookReceive } from "./webhook.ts";

/** The status of an accepted delivery (RFC 9110 15.3.3). */
const ACCEPTED = 202;

/** The status of a request of another method than a delivery's POST (RFC 9110 15.5.6). */
const METHOD_NOT_ALLOWED = 405;

/** A webhook as one installation answers it: each delivery verified, then received. */
export class WebhookServer {
    /** The served webhook. */
    readonly #webhook: Webhook;
    /** What takes the verified deliveries. */
    readonly #receive: WebhookReceive;
    /** The installation's resources, which bind the webhook's secret. */
    readonly #server: Pick<ServerContext, "resources">;

    /** Answer a webhook's deliveries for an installation. */
    constructor(
        webhook: Webhook,
        receive: WebhookReceive,
        server: Pick<ServerContext, "resources">,
    ) {
        this.#webhook = webhook;
        this.#receive = receive;
        this.#server = server;
    }

    /** Verify a delivery below the webhook's path with its secrets and hand it over. */
    readonly fetch: AdapterFetch = async (request, _caller, _context, store) => {
        // refuse a request that is no delivery
        if (request.method !== "POST") {
            return new Response(null, { status: METHOD_NOT_ALLOWED, headers: { allow: "POST" } });
        }

        try {
            // verify the delivery below the webhook's path with its secrets, then hand it over
            const { signature, template } = this.#webhook;
            const receivedAt = store.clock.now();
            const parameters = template.match(new URL(request.url).pathname);
            const delivery = await signature.verifier(store.environment)(
                request,
                await this.#secrets(),
                parameters,
                receivedAt,
            );
            await this.#receive(delivery, store);

            return new Response(null, { status: ACCEPTED });
        } catch (error) {
            return refusal(error, store.environment.telemetry);
        }
    };

    /** Read the bound secret's live versions, the current one first. */
    async #secrets(): Promise<readonly [string, ...string[]]> {
        const [current, ...previous] = await this.#server.resources
            .get(this.#webhook.secret)
            .versions();

        return [
            SecretReading.text(current),
            ...previous.map((reading) => SecretReading.text(reading)),
        ];
    }
}

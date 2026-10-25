import type { Clock } from "@destack/environment";
import type { Fetch } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { Duration, schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import type { MessageReason } from "../error/error.ts";
import type { PushSubscription } from "@destack/account/object";
import { type Content, type Destination, type Message, Refusal } from "../object/message.ts";

/** The statuses of a recipient that forgot the endpoint, 404 and 410 (RFC 8030 7.3). */
const GONE_STATUSES: ReadonlySet<number> = new Set([404, 410]);

/** The reasons a later try of the same message may get past. */
const RETRIED_REASONS: ReadonlySet<MessageReason> = new Set([
    "COMMAND_DEFERRED",
    "CREDENTIALS_EXPIRED",
    "PROVIDER_THROTTLED",
    "PROVIDER_TIMEOUT",
    "PROVIDER_UNAVAILABLE",
    "PROVIDER_UNREACHABLE",
]);

/** The reasons of statuses asking for a later try: a timeout, a request too early, a throttle. */
const RETRIED_STATUSES: ReadonlyMap<number, MessageReason> = new Map([
    [408, "PROVIDER_TIMEOUT"],
    [425, "PROVIDER_UNAVAILABLE"],
    [429, "PROVIDER_THROTTLED"],
]);

/** An HTTP date in the IMF-fixdate form senders generate (RFC 9110 5.6.7). */
const HTTP_DATE = /^[A-Z][a-z]{2}, \d{2} [A-Z][a-z]{2} \d{4} \d{2}:\d{2}:\d{2} GMT$/u;

/** The least status of a server's failure, which a later try may get past (RFC 9110 15.6). */
const SERVER_ERROR = 500;

/** What a provider made of one message: sent, deferred to a later try, or failed for good. */
const outcome = schema.discriminatedUnion("kind", [
    schema.object({ kind: schema.literal("sent") }),
    schema.object({
        kind: schema.literal("deferred"),
        /** The wait the provider asks for before the next try, in milliseconds. */
        after: schema.number().exactOptional(),
        /** Why the provider refused it for now. */
        refusal: Refusal,
    }),
    schema.object({
        kind: schema.literal("failed"),
        /** Why the provider refused it for good. */
        refusal: Refusal,
    }),
]);
/** What a provider made of one message. */
export type Outcome = schema.Infer<typeof outcome>;

/** What a provider made of one message, and how an HTTP answer reads as one. */
export const Outcome = Object.assign(outcome, {
    /** Retry a refusal a later try may get past, after a wait it asks for, and fail any other. */
    refuse(refusal: Refusal, after?: number): Outcome {
        // fail a refusal for good
        if (!RETRIED_REASONS.has(refusal.reason)) {
            return { kind: "failed", refusal };
        }

        return after === undefined
            ? { kind: "deferred", refusal }
            : { kind: "deferred", after, refusal };
    },

    /** Read an HTTP answer as sent, or refused by its status after its Retry-After. */
    read(response: Response, now: number): Outcome {
        // send on success
        const { status } = response;
        if (response.ok) {
            return { kind: "sent" };
        }

        // refuse a forgotten endpoint, a transient refusal or any other by its status
        const reason = GONE_STATUSES.has(status)
            ? "RECIPIENT_GONE"
            : (RETRIED_STATUSES.get(status) ??
              (status >= SERVER_ERROR ? "PROVIDER_UNAVAILABLE" : "MESSAGE_REJECTED"));
        const refusal = { reason, code: String(status), message: response.statusText };

        return Outcome.refuse(refusal, retryAfter(response.headers.get("Retry-After"), now));
    },

    /** Send a request and read its answer, retrying one the network lost or its signal ended. */
    async post(fetch: Fetch, request: Request, clock: Clock): Promise<Outcome> {
        // send the request
        const sent = await fetch(request).then(
            (response) => ({ response }),
            (failure: unknown) => ({ failure }),
        );
        if ("response" in sent) {
            return Outcome.read(sent.response, clock.now());
        }

        // retry a timeout or a network failure
        const { failure } = sent;
        if (request.signal.aborted && failure === request.signal.reason) {
            return Outcome.refuse({ reason: "PROVIDER_TIMEOUT", message: describe(failure) });
        } else if (failure instanceof TypeError) {
            return Outcome.refuse({ reason: "PROVIDER_UNREACHABLE", message: failure.message });
        }
        throw failure;
    },
});

/** Where a provider sends: a message's destination, a browser's with its push subscription. */
export type Delivery =
    | Exclude<Destination, { readonly channel: "push" }>
    | (Extract<Destination, { readonly channel: "push" }> & PushSubscription);

/** A provider sending one channel's messages, such as a mail transport or Standard Webhooks. */
export interface MessageProvider {
    /** Send one message with its opened content, keyed by its identifier to deliver it once. */
    readonly send: (
        message: Pick<Message, "id" | "scope" | "createdAt"> & {
            /** Where it goes. */
            readonly to: Delivery;
            /** The object it is sent for, a notification or an endpoint. */
            readonly source: ObjectReference;
            /** What it says. */
            readonly content: Content;
        },
        store: Pick<ObjectStore, "query" | "environment" | "as">,
    ) => Promise<Outcome>;
}

/** Describe a failure by its message, or as the value it is. */
function describe(failure: unknown): string {
    return failure instanceof Error ? failure.message : String(failure);
}

/** Read a Retry-After header as milliseconds from now, absent when invalid (RFC 9110 10.2.3). */
function retryAfter(header: string | null, now: number): number | undefined {
    // read the wait
    const date = header !== null && HTTP_DATE.test(header) ? Date.parse(header) : Number.NaN;
    if (header !== null && /^\d+$/u.test(header)) {
        return Duration.milliseconds({ seconds: Number(header) });
    } else if (Number.isFinite(date)) {
        return Math.max(0, date - now);
    } else {
        return undefined;
    }
}

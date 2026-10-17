import { defineSchema, Duration, schema } from "@destack/schema";
import { type Content, GONE, type Message, MessageError } from "../object/message.ts";
import type { ObjectStore } from "@destack/object/server";
import type { ObjectReference } from "@destack/sync";

/** The statuses of a recipient that forgot the endpoint, 404 and 410 (RFC 8030 7.3). */
const GONE_STATUSES: ReadonlySet<number> = new Set([404, 410]);

/** The statuses asking for a later try: a timeout, a request too early and throttling. */
const RETRIED_STATUSES: ReadonlySet<number> = new Set([408, 425, 429]);

/** What a provider made of one message: sent, retried, or refused for good. */
const outcome = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({ kind: schema.literal("sent") }),
        schema.object({
            kind: schema.literal("retry"),
            /** The wait the provider asks for before the next try, in milliseconds. */
            after: schema.number().exactOptional(),
            /** Why the provider refused it for now. */
            error: MessageError,
        }),
        schema.object({
            kind: schema.literal("failed"),
            /** Why the provider refused it for good. */
            error: MessageError,
        }),
    ]),
);
/** What a provider made of one message. */
export type Outcome = schema.Infer<typeof outcome>;

/** What a provider made of one message, and how an HTTP answer reads as one. */
export const Outcome = Object.assign(outcome, {
    /** Read an HTTP answer as sent, forgotten, retried after its Retry-After, or failed. */
    read(response: Response, now: number): Outcome {
        // send on success and fail for good at a forgotten endpoint
        const error = { code: String(response.status), message: response.statusText };
        if (response.ok) {
            return { kind: "sent" };
        } else if (GONE_STATUSES.has(response.status)) {
            return { kind: "failed", error: { ...error, code: GONE } };
        }
        // retry a timeout or a throttle or a server failure after the wait the answer gives
        else if (RETRIED_STATUSES.has(response.status) || response.status >= 500) {
            const after = retryAfter(response.headers.get("Retry-After"), now);

            return after === undefined ? { kind: "retry", error } : { kind: "retry", after, error };
        }

        return { kind: "failed", error };
    },
});

/** A provider sending one channel's messages, such as a mail transport or Standard Webhooks. */
export interface MessageProvider {
    /** Send one message with its opened content, keyed by its identifier to deliver it once. */
    readonly send: (
        message: Pick<Message, "id" | "scope" | "createdAt" | "to"> & {
            /** The object it is sent for, a notification or an endpoint. */
            readonly source: ObjectReference;
            /** What it says. */
            readonly content: Content;
        },
        store: Pick<ObjectStore, "query" | "clock" | "serving">,
    ) => Promise<Outcome>;
}

/** Read a Retry-After header as milliseconds from now, absent without one. */
function retryAfter(header: string | null, now: number): number | undefined {
    // read the header as seconds or as a date
    const seconds = Number(header ?? Number.NaN);
    const date = header === null ? Number.NaN : Date.parse(header);

    // wait the header's seconds or until its date
    if (Number.isFinite(seconds)) {
        return Duration.milliseconds({ seconds });
    } else if (Number.isFinite(date)) {
        return Math.max(0, date - now);
    }

    return undefined;
}

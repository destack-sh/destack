import type { CallServer } from "@destack/object";
import { defineSchema, Duration, schema } from "@destack/schema";
import { type Content, type Message, MessageError } from "../object/message.ts";

/** The code of a refusal for good from a recipient that forgot the endpoint (RFC 8030 7.3). */
export const GONE = "gone";

/** The statuses of a recipient that forgot the endpoint, 404 and 410 (RFC 8030 7.3). */
const GONE_STATUSES: ReadonlySet<number> = new Set([404, 410]);

/** The statuses of a recipient asking for a later try: a timeout, a request too early and throttling (RFC 9110 15.5.9, RFC 8470 5.2, RFC 6585 4). */
const RETRIED_STATUSES: ReadonlySet<number> = new Set([408, 425, 429]);

/** What a provider made of one message: sent, refused for a while and retried after a wait it may give in milliseconds, or refused for good. */
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
    /** Read an HTTP answer as sent, forgotten, retried after its Retry-After counted from a time, or failed. */
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

/** A provider sending the messages of one channel, such as a mail transport for email or Standard Webhooks for webhooks. */
export interface MessageProvider {
    /** Send one message with its opened content, keyed by its identifier so a repeated send delivers it once, reading what it needs through the space's server. */
    send(
        message: Pick<Message, "id" | "scope" | "createdAt" | "to"> & {
            /** What it says. */
            readonly content: Content;
        },
        server: Pick<CallServer, "query" | "clock">,
    ): Promise<Outcome>;
}

/** Read a Retry-After header as milliseconds from now, in seconds or as an HTTP date (RFC 9110 10.2.3), absent without one. */
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

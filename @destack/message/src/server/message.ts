import { ObjectReference } from "@destack/sync";
import { count, eq } from "@destack/db";
import type { Deriver } from "@destack/identity";
import type { ObjectReconciliation } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { aligned, present } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { endpoint } from "../object/endpoint.ts";
import { RetryPolicy } from "@destack/runtime/timer";
import { type Message, message, messageAttempt, MessageKey } from "../object/index.ts";
import type { MessageProvider, Outcome } from "../provider/index.ts";
import type { PushProvider } from "../push/index.ts";

/** How refused sends retry by default: 5 s doubling to 15 min, 8 attempts over about an hour. */
const RETRY = RetryPolicy.of({
    initialInterval: 5_000,
    maximumInterval: 15 * 60_000,
    maximumAttempts: 8,
    jitter: "full",
});

/** The messages sent at once by default: 16 in flight at 50 to 500 ms send 30 to 320 a second. */
const CONCURRENCY = 16;

/** How an installation sends its messages: each channel's providers and its keys. */
export interface MessageOptions {
    /** The provider of each channel, failing a message on a channel without one. */
    readonly providers: {
        /** The provider of email messages. */
        readonly email?: MessageProvider;
        /** The provider of push messages, which also answers the key browsers subscribe with. */
        readonly push?: PushProvider;
        /** The provider of webhook messages. */
        readonly webhook?: MessageProvider;
    };
    /** The installation's keys, whose content key opens its messages' content. */
    readonly keys: Pick<Deriver, "derivePrivateKey">;
    /** How refused sends retry, about an hour of eight attempts by default. */
    readonly retry?: RetryPolicy;
}

/** Implement messages: a controller sends each through its channel's provider. */
export function implementMessage(options: MessageOptions) {
    const sending = { ...options, retry: options.retry ?? RETRY };

    return message.implement({
        methods: {
            create: async (call, next) => {
                // let only an endpoint's managers post webhooks to it, with its secret
                const { to, ciphertext } = call.input;
                if (to.channel === "webhook") {
                    const source = ObjectReference.at("source", call.input);
                    await call.require(endpoint.permission("manage"), source);
                }

                // show a desktop message at once, without content
                if (to.channel === "desktop") {
                    return next(
                        call.with({
                            input: { ...call.input, status: "sent", settledAt: call.now },
                        }),
                    );
                }
                // refuse a provider's message without its content
                else if (ciphertext === undefined) {
                    throw new CallError("BAD_REQUEST", {
                        message: `a message on ${to.channel} needs its sealed content`,
                    });
                }

                return next();
            },
            serverKey: async () => {
                // refuse pushes on a machine without a push provider
                const push = options.providers.push;
                if (push === undefined) {
                    throw new CallError("NOT_FOUND", {
                        message: "this machine does not send pushes",
                    });
                }

                return { key: await push.applicationServerKey() };
            },
            contentKey: async () => ({ key: (await MessageKey.derive(options.keys)).key }),
        },
        controller: {
            // send pending messages
            pending: { status: "pending" },
            concurrency: CONCURRENCY,
            reconcile: (reconciliation) => reconcile(reconciliation, sending),
        },
    });
}

/** Send a due message and record its outcome, dropping its sealed content once it settles. */
async function reconcile(
    reconciliation: ObjectReconciliation<typeof message>,
    options: Required<MessageOptions>,
): Promise<number | undefined> {
    // read the key's one message whole at its current revision
    const { now } = reconciliation;
    const row = aligned(reconciliation.rows, 0);
    const whole = aligned(
        await reconciliation.database
            .select()
            .from(message.table)
            .where(eq(message.table.id, row.id)),
        0,
    );

    // wait for a refused message's next attempt
    if (whole.retryAt !== null && whole.retryAt > now) {
        return whole.retryAt - now;
    }
    // send it and settle it as sent, retried after the policy's interval, or failed once the policy gives up
    else {
        const outcome = await attempt(options, whole, reconciliation.store);
        const attempts = await recordAttempt(reconciliation, whole, outcome);
        await observe(reconciliation, whole, settle(outcome, options.retry, attempts, now));
    }

    return undefined;
}

/** Record the controller's observation of a message: its written fields and Sent condition. */
async function observe(
    reconciliation: ObjectReconciliation<typeof message>,
    row: Message,
    observed: ReturnType<typeof settle>,
): Promise<void> {
    await reconciliation.mutate([
        message.call("observe", {
            ...message.target(row),
            observedGeneration: row.generation,
            conditions: { Sent: observed.sent },
            fields: observed.fields,
        }),
    ]);
}

/** Open a message's content and send it through its channel's provider, if one exists here. */
async function attempt(
    options: Pick<MessageOptions, "providers" | "keys">,
    whole: Message,
    store: ObjectStore,
): Promise<Outcome> {
    // fail a message no provider here sends, a desktop message among them
    const { channel } = whole.to;
    const provider = channel === "desktop" ? undefined : options.providers[channel];
    if (provider === undefined) {
        return {
            kind: "failed",
            error: {
                code: "unprovided",
                message: `no provider sends ${channel} messages here`,
            },
        };
    }

    // open the content right before the provider sends it
    const sealed = present(whole.ciphertext, "an unsettled message's ciphertext");
    const key = await MessageKey.derive(options.keys);
    const content = await key.open(sealed, whole.scope);

    return provider.send({ ...whole, source: ObjectReference.at("source", whole), content }, store);
}

/** Record a try of a message, returning how many tries it has. */
async function recordAttempt(
    reconciliation: ObjectReconciliation<typeof message>,
    whole: Pick<Message, "id" | "scope">,
    outcome: Outcome,
): Promise<number> {
    // record the try
    await reconciliation.mutate([
        messageAttempt.call("create", {
            ...messageAttempt.in(whole.scope),
            parentId: whole.id,
            outcome,
        }),
    ]);

    // count the tries so far
    const [tried] = await reconciliation.database
        .select({ count: count() })
        .from(messageAttempt.table)
        .where(eq(messageAttempt.table.parentId, whole.id));

    return present(tried, "a count's row").count;
}

/** Decide a message's fields and Sent condition after a try. */
function settle(outcome: Outcome, retry: RetryPolicy, attempts: number, now: number) {
    // settle a sent message
    if (outcome.kind === "sent") {
        return {
            fields: { status: "sent" as const, settledAt: now, retryAt: null, ciphertext: null },
            sent: { status: "true" as const, reason: "Sent", message: "" },
        };
    }
    // retry after the provider's wait or the policy's interval while the policy retries
    else if (outcome.kind === "retry" && RetryPolicy.isRetried(retry, attempts)) {
        const retryAt = now + (outcome.after ?? RetryPolicy.interval(retry, attempts));

        return {
            fields: { retryAt },
            sent: {
                status: "unknown" as const,
                reason: "Retrying",
                message: outcome.error.message,
            },
        };
    }
    // fail a refusal for good or one the policy no longer retries
    else {
        return {
            fields: {
                status: "failed" as const,
                reason: outcome.error.code,
                settledAt: now,
                retryAt: null,
                ciphertext: null,
            },
            sent: {
                status: "false" as const,
                reason: outcome.error.code,
                message: outcome.error.message,
            },
        };
    }
}

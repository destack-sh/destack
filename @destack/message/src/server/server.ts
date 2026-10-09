import { ServerCall, type CallServer } from "@destack/object";
import { count, eq } from "@destack/db";
import type { Deriver } from "@destack/identity";
import type { ObjectReconciliation } from "@destack/object";
import type { Extension } from "@destack/object/server";
import { implement, type ServiceContext } from "@destack/service/server";
import { aligned, Duration, present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { RetryPolicy } from "@destack/service/timer";
import { type Message, message, messageAttempt, MessageKey } from "../object/index.ts";
import type { MessageProvider, Outcome } from "../provider/index.ts";
import type { PushProvider } from "../push/index.ts";
import { messageService } from "../service/index.ts";
import { type EndpointOptions, serveEndpoints } from "./endpoint.ts";

/** How refused sends retry by default: 5 s doubling to 15 min, 8 attempts over about an hour. */
const RETRY = RetryPolicy.of({
    initialInterval: 5_000,
    maximumInterval: 15 * 60_000,
    maximumAttempts: 8,
    jitter: "full",
});

/** How long a settled message keeps its sealed content by default: seven days after delivery or a final failure, long enough to inspect a delivery, as webhook services keep payloads for a retention window. */
const SEALED_RETENTION: Duration = { days: 7 };

/** The messages sent at once by default: 16 in flight at 50 to 500 ms send 30 to 320 a second. */
const CONCURRENCY = 16;

/** What a machine serves its spaces' messages with: the providers of each channel and the deriver opening their sealed content. */
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
    /** Read the deriver of the identity holding a scope, whose message key opens the content sealed to the scope's messages. */
    readonly deriver: (scope: string) => Promise<Pick<Deriver, "derivePrivateKey">>;
    /** The events keeping the calls the space's endpoints deliver. */
    readonly events: EndpointOptions["events"];
    /** How refused sends retry, about an hour of eight attempts by default. */
    readonly retry?: RetryPolicy;
    /** How long a settled message keeps its sealed content, seven days by default. */
    readonly retention?: Duration;
}

/** Implement the message service: keep a space's messages and send each through its channel's provider, deliver its endpoints' histories, and answer the key its messages are sealed to and the key each space's browsers subscribe with. */
export function implementMessages(options: MessageOptions): Extension {
    // route the keys and pushes, the push provider answering the key browsers subscribe with
    const pushes = implement(messageService.router.push).$context<ServiceContext>();
    const keys = implement(messageService.router.key).$context<ServiceContext>();
    const push = options.providers.push;
    const key = async (scope: string) =>
        (await MessageKey.derive(await options.deriver(scope))).key;

    return {
        service: messageService,
        objects: {
            ...serveMessages(options),
            endpoint: serveEndpoints({ events: options.events, key }),
        },
        serve: () => ({
            // route the message service's keys and pushes beside its objects
            procedures: {
                key: keys.router({
                    read: keys.read.handler(async ({ input }) => ({ key: await key(input.scope) })),
                }),
                push: pushes.router({
                    applicationServerKey: pushes.applicationServerKey.handler(async ({ input }) => {
                        // refuse a machine sending no pushes
                        if (push === undefined) {
                            throw new ServiceError("NOT_FOUND", {
                                message: "this machine sends no pushes",
                            });
                        }

                        return { key: await push.applicationServerKey(input.scope) };
                    }),
                }),
            },
        }),
    };
}

/** Serve messages: send each one through its channel's provider under a controller, opening its sealed content only then, and erase the content once its retention passes. */
export function serveMessages(
    options: Pick<MessageOptions, "providers" | "deriver" | "retry" | "retention">,
) {
    const sending = {
        providers: options.providers,
        deriver: options.deriver,
        retry: options.retry ?? RETRY,
        retention: Duration.milliseconds(options.retention ?? SEALED_RETENTION),
    };

    return {
        message: message.control({
            pending: { erasedAt: { isNull: true } },
            concurrency: CONCURRENCY,
            reconcile: (reconciliation) => send(reconciliation, sending),
        }),
        attempt: messageAttempt,
    };
}

/** Send each due message and record what its provider made of it, or erase a settled one's content once its retention passes, returning when to look again. */
async function send(
    reconciliation: ObjectReconciliation<typeof message>,
    options: {
        readonly providers: MessageOptions["providers"];
        readonly deriver: MessageOptions["deriver"];
        readonly retry: RetryPolicy;
        readonly retention: number;
    },
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

    // erase a settled message's content once its retention passes
    if (whole.status !== "pending") {
        const settledAt = present(whole.settledAt, "a settled message's time");
        const erasedAt = settledAt + options.retention;
        if (erasedAt > now) {
            return erasedAt - now;
        }
        await reconciliation.execute(message, "erase", [ServerCall.of(whole)]);
    }
    // wait for a refused message's next attempt
    else if (whole.retryAt !== null && whole.retryAt > now) {
        return whole.retryAt - now;
    }
    // send it and settle it as sent, retried after the policy's interval, or failed once the policy gives up
    else {
        const outcome = await attempt(options, whole, reconciliation.server);
        const attempts = await recordAttempt(reconciliation, whole, outcome);
        await observe(reconciliation, whole, settle(outcome, options.retry, attempts, now));
    }

    return undefined;
}

/** Record the controller's observation of a message: the fields it writes and its Sent condition. */
async function observe(
    reconciliation: ObjectReconciliation<typeof message>,
    row: Message,
    observed: ReturnType<typeof settle>,
): Promise<void> {
    await reconciliation.execute(message, "observe", [
        ServerCall.of(row, {
            observedGeneration: row.generation,
            conditions: { Sent: observed.sent },
            fields: observed.fields,
        }),
    ]);
}

/** Open a message's content and send it through its channel's provider, failing one no provider here sends. */
async function attempt(
    options: {
        readonly providers: MessageOptions["providers"];
        readonly deriver: MessageOptions["deriver"];
    },
    whole: Message,
    server: CallServer,
): Promise<Outcome> {
    // fail a message no provider here sends
    const provider = options.providers[whole.to.channel];
    if (provider === undefined) {
        return {
            kind: "failed",
            error: {
                code: "unprovided",
                message: `no provider sends ${whole.to.channel} messages here`,
            },
        };
    }

    // open the content right before the provider sends it
    const sealed = present(whole.ciphertext, "an unsettled message's ciphertext");
    const key = await MessageKey.derive(await options.deriver(whole.scope));
    const content = await key.open(sealed, whole.scope);

    return provider.send({ ...whole, content }, server);
}

/** Record a try of a message, returning how many tries it has. */
async function recordAttempt(
    reconciliation: ObjectReconciliation<typeof message>,
    whole: Pick<Message, "id" | "scope">,
    outcome: Outcome,
): Promise<number> {
    // record the try
    await reconciliation.execute(messageAttempt, "create", [
        { scope: whole.scope, input: { parentId: whole.id, outcome } },
    ]);

    // count the tries so far
    const [tried] = await reconciliation.database
        .select({ count: count() })
        .from(messageAttempt.table)
        .where(eq(messageAttempt.table.parentId, whole.id));

    return present(tried, "a count's row").count;
}

/** Decide a message's fields and Sent condition after a try: sent, retried after the policy's interval, or failed once the policy gives up. */
function settle(outcome: Outcome, retry: RetryPolicy, attempts: number, now: number) {
    // settle a sent message
    if (outcome.kind === "sent") {
        return {
            fields: { status: "sent" as const, settledAt: now, retryAt: null },
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
            fields: { status: "failed" as const, settledAt: now, retryAt: null },
            sent: {
                status: "false" as const,
                reason: outcome.error.code,
                message: outcome.error.message,
            },
        };
    }
}

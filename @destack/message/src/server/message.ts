import { AccessContext, Caller, Grant, principal } from "@destack/access";
import { client, type Recipient, user } from "@destack/account/object";
import { ObjectReference, Scope } from "@destack/sync";
import { count, eq, Snapshot } from "@destack/db";
import type { Deriver } from "@destack/identity";
import type { ObjectReconciliation } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { nth, present, schema } from "@destack/schema";
import { endpoint } from "../object/endpoint.ts";
import { RetryPolicy } from "@destack/environment";
import { type Message, message, messageAttempt, MessageKey, task } from "../object/index.ts";
import type { Delivery, MessageProvider, Outcome } from "../provider/index.ts";
import type { PushProvider } from "../push/index.ts";
import { MessageError } from "../error/error.ts";

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
    /** The provider of each channel served here, refusing a message on a channel without one. */
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
                // refuse a message on a channel no provider here serves
                const { to, ciphertext } = call.input;
                if (to.channel !== "desktop" && options.providers[to.channel] === undefined) {
                    throw MessageError.of(
                        "CHANNEL_UNSERVED",
                        `this machine does not send ${to.channel} messages`,
                    );
                }

                // let only an endpoint's managers post webhooks to it, with its secret
                if (to.channel === "webhook") {
                    const source = ObjectReference.at("source", call.input);
                    await call.require(endpoint.permission("manage"), source);
                }
                // push only from the person's home, to a browser of theirs subscribed to pushes
                else if (to.channel === "push") {
                    const recipient = await call.query(
                        await call.delegated(),
                        user.call("recipient", { id: to.user }),
                    );
                    if (findBrowser(recipient, to.client) === undefined) {
                        throw MessageError.of(
                            "RECIPIENT_GONE",
                            `client ${to.client} receives no pushes`,
                        );
                    }
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
                    throw MessageError.of(
                        "SEALED_CONTENT_MISSING",
                        `a message on ${to.channel} is missing its encrypted content`,
                    );
                }

                return next();
            },
            serverKey: async () => {
                // refuse pushes on a machine without a push provider
                const push = options.providers.push;
                if (push === undefined) {
                    throw MessageError.of(
                        "CHANNEL_UNSERVED",
                        "this machine does not send push messages",
                    );
                }

                return { key: await push.applicationServerKey() };
            },
            contentKey: async (context) => ({
                key: (await MessageKey.derive(options.keys, context.environment)).key,
            }),
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
    const row = nth(reconciliation.rows, 0);
    const whole = nth(
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
    // send it and record the try, then look again once a later try is due
    else {
        const attempts = (await countAttempts(reconciliation, whole)) + 1;
        const outcome = await attempt(options, whole, reconciliation.store);
        const drawn = RetryPolicy.draw(`${whole.id} ${attempts}`);
        const settled = settle(outcome, options.retry, attempts, now, drawn);
        await observe(reconciliation, whole, outcome, settled);
        const { retryAt } = settled.fields;

        return retryAt === null ? undefined : retryAt - now;
    }
}

/** Record a try of a message with the controller's observation: its fields and Sent condition. */
async function observe(
    reconciliation: ObjectReconciliation<typeof message>,
    row: Message,
    outcome: Outcome,
    observed: ReturnType<typeof settle>,
): Promise<void> {
    // forget a browser its push service forgot, as the space its recipient lets read its clients
    const { to } = row;
    const isGone = to.channel === "push" && observed.fields.reason === "RECIPIENT_GONE";
    const forget = isGone
        ? [
              task.start(
                  { call: client.call("unsubscribe", { id: to.client, userId: to.user }) },
                  {
                      actor: principal.space.reference(
                          Scope.universe.id,
                          schema.identifier("space").parse(row.scope),
                      ),
                  },
              ),
          ]
        : [];

    // record the try and the observation with it, starting the task in the message's space
    await reconciliation.mutate(
        [
            messageAttempt.call("create", {
                ...messageAttempt.in(row.scope),
                parentId: row.id,
                outcome,
            }),
            message.call("observe", {
                ...message.target(row),
                observedGeneration: row.generation,
                conditions: { Sent: observed.sent },
                fields: observed.fields,
            }),
            ...forget,
        ],
        { scope: row.scope },
    );
}

/** Find a person's browser subscribed to pushes, absent for a client that is none. */
function findBrowser(recipient: Recipient, id: string): Recipient["browsers"][number] | undefined {
    return recipient.browsers.find((each) => each.id === id);
}

/** Read where a message goes now, a browser with its subscription, absent once unsubscribed. */
async function deliver(whole: Message, store: ObjectStore): Promise<Delivery | undefined> {
    // pass every destination but a browser on
    const { to } = whole;
    if (to.channel !== "push") {
        return to;
    }

    // read the browser's subscription for the person's home, the message's space
    const now = store.clock.now();
    const delegator = principal.space.reference(Scope.universe.id, whole.scope);
    const delegate = Caller.principal(store.as(message, now));
    const caller = await Grant.act(Snapshot.live(store.database), { delegator, delegate }, now);
    const call = user.call("recipient", { id: to.user });
    const recipient = await store.query(AccessContext.of(caller, now), call);
    const browser = findBrowser(recipient, to.client);

    return browser === undefined ? undefined : { ...to, url: browser.url, keys: browser.keys };
}

/** Open a message's content and send it through its channel's provider, if one exists here. */
async function attempt(
    options: Pick<MessageOptions, "providers" | "keys">,
    whole: Message,
    store: ObjectStore,
): Promise<Outcome> {
    // fail a message whose channel this machine stopped serving since its creation
    const { channel } = whole.to;
    const provider = channel === "desktop" ? undefined : options.providers[channel];
    if (provider === undefined) {
        return {
            kind: "failed",
            refusal: {
                reason: "CHANNEL_UNSERVED",
                message: `this machine does not send ${channel} messages`,
            },
        };
    }

    // fail a message to a browser no longer subscribed
    const delivery = await deliver(whole, store);
    if (delivery === undefined) {
        const refusal = {
            reason: "RECIPIENT_GONE" as const,
            message: "the browser receives no pushes",
        };

        return { kind: "failed", refusal };
    }

    // open the content right before the provider sends it
    const sealed = present(whole.ciphertext, "an unsettled message's ciphertext");
    const key = await MessageKey.derive(options.keys, store.environment);
    const content = await key.open(sealed, whole.scope);
    const source = ObjectReference.at("source", whole);

    return provider.send({ ...whole, to: delivery, source, content }, store);
}

/** Count a message's recorded tries. */
async function countAttempts(
    reconciliation: ObjectReconciliation<typeof message>,
    whole: Pick<Message, "id">,
): Promise<number> {
    const [tried] = await reconciliation.database
        .select({ count: count() })
        .from(messageAttempt.table)
        .where(eq(messageAttempt.table.parentId, whole.id));

    return present(tried, "a count's row").count;
}

/** Decide a message's fields and Sent condition after a try. */
function settle(
    outcome: Outcome,
    retry: RetryPolicy,
    attempts: number,
    now: number,
    drawn: number,
) {
    // settle a sent message
    if (outcome.kind === "sent") {
        return {
            fields: { status: "sent" as const, settledAt: now, retryAt: null, ciphertext: null },
            sent: { status: "true" as const, reason: "Sent", message: "" },
        };
    }
    // try again after the provider's wait or the policy's interval, while the policy allows
    else if (outcome.kind === "deferred" && RetryPolicy.isRetried(retry, attempts)) {
        const wait =
            outcome.after === undefined
                ? RetryPolicy.interval(retry, attempts, () => drawn)
                : Math.min(outcome.after, retry.maximumInterval);
        const retryAt = now + Math.ceil(wait);

        return {
            fields: { retryAt },
            sent: {
                status: "unknown" as const,
                reason: "Retrying",
                message: outcome.refusal.message,
            },
        };
    }
    // fail a refusal for good or one the policy no longer retries
    else {
        return {
            fields: {
                status: "failed" as const,
                reason: outcome.refusal.reason,
                settledAt: now,
                retryAt: null,
                ciphertext: null,
            },
            sent: {
                status: "false" as const,
                reason: "Failed",
                message: `${outcome.refusal.reason}: ${outcome.refusal.message}`,
            },
        };
    }
}

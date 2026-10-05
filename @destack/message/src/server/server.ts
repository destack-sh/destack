import { count, type DatabaseConnection, eq } from "@destack/db";
import type { ObjectReconciliation } from "@destack/object";
import { ObjectServer, Subscriber } from "@destack/object/server";
import type { CallKey } from "@destack/service/request";
import type { ServiceImplementation } from "@destack/service/server";
import { RetryPolicy } from "@destack/service/timer";
import { space } from "@destack/space/object";
import type { Uplink } from "@destack/sync";
import { message, messageAttempt } from "../object/index.ts";
import type { MessageProvider } from "../provider/index.ts";
import { messageService } from "../service/index.ts";

/** How refused sends retry by default: 5 s doubling to 15 min, 8 attempts over about an hour. */
const RETRY = RetryPolicy.of({
    initialInterval: 5_000,
    maximumInterval: 15 * 60_000,
    maximumAttempts: 8,
    jitter: "full",
});

/** The messages sent at once by default: 16 in flight at 50 to 500 ms send 30 to 320 a second. */
const CONCURRENCY = 16;

/** What a cell serves its spaces' messages with: their database, the providers of each channel and the spaces it follows. */
export interface MessageOptions {
    /** The database keeping the messages, with copies of the spaces they live in. */
    readonly database: DatabaseConnection;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The providers sending each channel's messages. */
    readonly providers: readonly MessageProvider[];
    /** The cell serving the spaces, which with the service's name names the copy of their rows. */
    readonly cell: string;
    /** The space service's uplink, streaming the spaces' rows and their chains and receiving their changes. */
    readonly spaces: Uplink;
    /** How refused sends retry, about an hour of eight attempts by default. */
    readonly retry?: RetryPolicy;
}

/** The message service with the object server keeping the messages. */
export interface MessageImplementation extends ServiceImplementation {
    /** The object server keeping the messages, a source of the space service's copies. */
    readonly objects: ObjectServer;
}

/** Implement the message service: keep a cell's spaces' messages and send each through its channel's provider. */
export function implementMessages(options: MessageOptions): MessageImplementation {
    const objects: ObjectServer = new ObjectServer({
        objects: serveMessages(options.providers, options.retry ?? RETRY),
        policies: [space],
        database: options.database,
        callKey: options.callKey,
        origin: { package: messageService.package, service: messageService.name },
        subscriber: Subscriber.of(options.spaces, () =>
            objects.source.workloadSubscriptions(`${options.cell}/${messageService.name}`),
        ),
    });

    return { ...objects.implement(messageService), objects };
}

/** Serve messages, sending each unsettled one through its channel's provider under a controller, and the attempts it records. */
export function serveMessages(providers: readonly MessageProvider[], retry: RetryPolicy) {
    return {
        message: message.control({
            pending: { sentAt: { isNull: true }, failedAt: { isNull: true } },
            concurrency: CONCURRENCY,
            reconcile: (reconciliation) => send(reconciliation, providers, retry),
        }),
        messageAttempt,
    };
}

/** Send one due message and record what its provider made of it, returning when to try again. */
async function send(
    reconciliation: ObjectReconciliation<typeof message>,
    providers: readonly MessageProvider[],
    retry: RetryPolicy,
): Promise<number | undefined> {
    const { now } = reconciliation;
    for (const row of reconciliation.rows) {
        // wait for a refused message's next attempt
        if (row.retryAt !== null && row.retryAt > now) {
            return row.retryAt - now;
        }

        // read the whole message, its sensitive secret included, and send it through its channel's provider
        const [whole] = await reconciliation.database
            .select()
            .from(message.table)
            .where(eq(message.table.id, row.id));
        if (whole === undefined) {
            continue;
        }
        const provider = providers.find((candidate) => candidate.channel === row.to.channel);
        const outcome =
            provider === undefined
                ? {
                      outcome: "failed" as const,
                      error: {
                          code: "unprovided",
                          message: `no provider sends ${row.to.channel} messages here`,
                      },
                  }
                : await provider.send(whole);

        // record the try, counting the refusals so far
        await reconciliation.server.executeAsSystem(
            messageAttempt,
            "record",
            [
                {
                    scope: whole.scope,
                    input: {
                        parentId: whole.id,
                        outcome: outcome.outcome,
                        ...(outcome.outcome === "sent" ? {} : { error: outcome.error }),
                    },
                },
            ],
            now,
        );
        const [tried] = await reconciliation.database
            .select({ count: count() })
            .from(messageAttempt.table)
            .where(eq(messageAttempt.table.parentId, whole.id));
        const attempts = tried?.count ?? 0;

        // settle it as sent, retry it after the policy's interval, or fail it once the policy gives up
        const isFailed =
            outcome.outcome === "failed" ||
            (outcome.outcome === "retry" && !RetryPolicy.isRetried(retry, attempts));
        const fields =
            outcome.outcome === "sent"
                ? { sentAt: now, retryAt: null }
                : isFailed
                  ? { failedAt: now, retryAt: null }
                  : {
                        retryAt:
                            now +
                            ((outcome.outcome === "retry" ? outcome.after : undefined) ??
                                RetryPolicy.interval(retry, attempts)),
                    };
        await reconciliation.server.executeAsSystem(
            message,
            "observe",
            [
                {
                    scope: whole.scope,
                    target: whole,
                    input: {
                        observedGeneration: whole.generation,
                        conditions: {
                            Sent:
                                outcome.outcome === "sent"
                                    ? { status: "true", reason: "Sent", message: "" }
                                    : {
                                          status: isFailed ? "false" : "unknown",
                                          reason: isFailed ? outcome.error.code : "Retrying",
                                          message: outcome.error.message,
                                      },
                        },
                        fields,
                    },
                },
            ],
            now,
        );
    }

    return undefined;
}

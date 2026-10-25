import type { Deriver } from "@destack/identity";
import { ObjectReference, RequestId, Subject } from "@destack/sync";
import { Caller } from "@destack/access";
import { and, type DatabaseConnection, desc, eq, inArray, isNull, ne } from "@destack/db";
import {
    CallEvent,
    call,
    type MethodContext,
    type ObjectReconciliation,
    ObjectWatch,
} from "@destack/object";
import { secret } from "@destack/vault/object";
import { type DigestCrypto, nth, Identifier, present } from "@destack/schema";
import {
    RESERVED_HEADERS,
    type WebhookAuthentication,
    type WebhookContent,
} from "../webhook/message.ts";
import { MessageError } from "../error/error.ts";
import {
    type Endpoint,
    endpoint,
    endpointConditions,
    type Message,
    message,
    messageConditions,
    MessageKey,
} from "../object/index.ts";

/** The calls on one page of an endpoint's history: a hundred 1 KB calls post one 100 KB batch. */
const PAGE_CALLS = 100;

/** The record type of a batch of calls. */
const RECORD_TYPE = "call";

/** Implement endpoints: a controller delivers each enabled endpoint's calls as webhook messages. */
export function implementEndpoint(keys: Pick<Deriver, "derivePrivateKey">) {
    return endpoint.implement({
        methods: {
            create: async (mutation, next) => {
                // let the installation consume the secret, as its creator may let software use it
                requireHeader(mutation.input.authentication);
                const created = await next();
                await consume(mutation, created.secret);

                return created;
            },
            update: async (mutation, next) => {
                // let the installation consume a new secret, and release the replaced one once unused
                if (mutation.input.authentication !== undefined) {
                    requireHeader(mutation.input.authentication);
                }
                const replaced = mutation.target.secret;
                const updated = await next();
                const set = mutation.input.secret;
                if (
                    set !== undefined &&
                    ObjectReference.key(set) !== ObjectReference.key(replaced)
                ) {
                    await consume(mutation, set);
                    await releaseUnused(mutation, replaced);
                }

                return updated;
            },
            delete: async (mutation, next) => {
                // release the secret once no other endpoint of the scope uses it
                const deleted = await next();
                await releaseUnused(mutation, mutation.target.secret);

                return deleted;
            },
        },
        controller: {
            pending: { status: "enabled", deletedAt: { isNull: true } },
            watches: [
                ObjectWatch.of(message.table, (row) => {
                    // wake the endpoint a sent message of it names
                    const source = ObjectReference.at("source", row);

                    return endpoint.is(source) && row.status !== "pending"
                        ? [{ id: source.id }]
                        : [];
                }),
                ObjectWatch.of(call.table, async (row, database) => {
                    // wake the live enabled endpoints of the scope with a succeeded call
                    if (call.eventKind().event(row).keys.outcome !== "success") {
                        return [];
                    }
                    const enabled = await database
                        .select({ id: endpoint.table.id })
                        .from(endpoint.table)
                        .where(
                            and(
                                eq(endpoint.table.scope, row.scope),
                                eq(endpoint.table.status, "enabled"),
                                isNull(endpoint.table.deletedAt),
                            ),
                        );

                    return enabled.map((entry) => ({ id: entry.id }));
                }),
            ],
            reconcile: (reconciliation) =>
                deliver(reconciliation, nth(reconciliation.rows, 0), keys),
        },
    });
}

/** Deliver an endpoint's next page of calls as messages, then advance its cursor and condition. */
async function deliver(
    reconciliation: ObjectReconciliation<typeof endpoint>,
    row: Endpoint,
    keys: Pick<Deriver, "derivePrivateKey">,
): Promise<number | undefined> {
    // read the succeeded calls of the endpoint's scope committed past its place, from its first read on
    const reader = endpoint.reader("endpoint").at(row.id);
    const after = await reader.after(reconciliation.database);
    const page = await reconciliation.store.events.changes(
        call.eventKind(),
        { scopes: [row.scope], where: { outcome: "success" } },
        after,
        PAGE_CALLS,
    );
    const items = page.events.map((event) => CallEvent.parse(event.data));

    // describe a message for each subscribed call, or one for the page
    const deliverer = present(
        Caller.principal(reconciliation.caller),
        "the delivering installation",
    );
    const calls = items.filter((item) => isSubscribed(row, item, deliverer));
    const key = await MessageKey.derive(keys, reconciliation.store.environment);
    const described = await describeMessages(
        row,
        calls,
        key,
        reconciliation.now,
        reconciliation.store.environment.crypto,
    );

    // create the messages a repeated page has yet to create
    const existing = await reconciliation.database
        .select({ id: message.table.id })
        .from(message.table)
        .where(
            inArray(
                message.table.id,
                described.map((entry) => entry.id),
            ),
        );
    const created = new Set<string>(existing.map((entry) => entry.id));
    const messages = described.filter((entry) => !created.has(entry.id));
    await reconciliation.mutate(
        messages.map((entry) =>
            message.call("create", {
                ...message.target({ scope: entry.scope, id: entry.id }),
                ...entry.input,
            }),
        ),
    );

    // move the endpoint's place past the page, and list the calls after it
    if (page.sequence > after) {
        await reader.move(after, page.sequence, reconciliation.now, reconciliation.database);
    }

    // mark the endpoint by its last settled message
    await mark(reconciliation, row);

    return items.length < PAGE_CALLS ? undefined : 0;
}

/** Mark an endpoint's Delivered condition by its last settled message, once that changes it. */
async function mark(
    reconciliation: ObjectReconciliation<typeof endpoint>,
    row: Endpoint,
): Promise<void> {
    // describe the condition its last settled message sets
    const settled = await lastSettled(reconciliation.database, row);
    if (settled === undefined) {
        return;
    }
    const delivered = deliveredCondition(settled);
    const current = endpointConditions.read(row, "Delivered");
    if (current?.status === delivered.status && current.message === delivered.message) {
        return;
    }

    // observe the changed condition
    await reconciliation.mutate([
        endpoint.call("observe", {
            ...endpoint.target(row),
            observedGeneration: row.generation,
            conditions: { Delivered: delivered },
        }),
    ]);
}

/** Report whether an endpoint receives a call: one of its events, by anyone but its deliverer. */
function isSubscribed(row: Endpoint, ended: CallEvent, deliverer: Subject): boolean {
    const isDelivery = Subject.same(ended.execution.context.caller.subject, deliverer);

    return !isDelivery && (row.events === "all" || row.events.includes(ended.method));
}

/** Describe the messages with a page's calls to an endpoint: an event per call, or one batch. */
async function describeMessages(
    row: Endpoint,
    calls: readonly CallEvent[],
    key: MessageKey,
    now: number,
    crypto: DigestCrypto,
): Promise<Awaited<ReturnType<typeof describeMessage>>[]> {
    // send nothing for a page without subscribed calls
    const [first] = calls;
    const last = calls.at(-1);
    if (first === undefined || last === undefined) {
        return [];
    }

    // send each call as an event
    if (row.format === "event") {
        return Promise.all(
            calls.map(async (ended) => {
                const { execution } = ended;

                return describeMessage(row, key, now, {
                    requestId: await RequestId.derive(
                        Math.max(row.updatedAt, execution.finishedAt ?? execution.startedAt),
                        `${row.id} ${row.generation} ${execution.id}`,
                        crypto,
                    ),
                    content: {
                        channel: "webhook",
                        type: ended.method,
                        format: "event",
                        data: record(ended),
                    },
                });
            }),
        );
    }

    // send the calls as one batch
    return [
        await describeMessage(row, key, now, {
            requestId: await RequestId.derive(
                Math.max(row.updatedAt, first.execution.finishedAt ?? first.execution.startedAt),
                `${row.id} ${row.generation} ${first.execution.id} ${last.execution.id}`,
                crypto,
            ),
            content: {
                channel: "webhook",
                type: RECORD_TYPE,
                format: row.format,
                data: calls.map((ended) => ({ type: ended.method, ...record(ended) })),
            },
        }),
    ];
}

/** Describe a call as an endpoint receives it: its scope, target and details. */
function record(ended: CallEvent) {
    const { execution } = ended;

    return { scope: execution.context.scope, target: execution.target, details: execution.details };
}

/** Describe one message to an endpoint, sealed to the installation's message key. */
async function describeMessage(
    row: Endpoint,
    key: MessageKey,
    now: number,
    sent: {
        readonly requestId: string;
        readonly content: WebhookContent;
    },
) {
    return {
        scope: row.scope,
        requestId: sent.requestId,
        id: Identifier.derive("message", sent.requestId),
        input: {
            to: { channel: "webhook" as const },
            ciphertext: await key.seal(sent.content, row.scope),
            ...ObjectReference.columns("source", endpoint.reference(row.scope, row.id)),
        },
    };
}

/** Read the last message to an endpoint its controller sent or gave up. */
async function lastSettled(
    database: DatabaseConnection,
    row: Endpoint,
): Promise<Pick<Message, "status" | "conditions"> | undefined> {
    // read the newest settled message on the source index
    const source = ObjectReference.columns("source", endpoint.reference(row.scope, row.id));
    const [last] = await database
        .select({ status: message.table.status, conditions: message.table.conditions })
        .from(message.table)
        .where(
            and(
                eq(message.table.sourcePackageId, source.sourcePackageId),
                eq(message.table.sourceType, source.sourceType),
                eq(message.table.sourceScope, source.sourceScope),
                eq(message.table.sourceId, source.sourceId),
                ne(message.table.status, "pending"),
            ),
        )
        .orderBy(desc(message.table.createdAt), desc(message.table.id))
        .limit(1);

    return last;
}

/** Describe an endpoint's Delivered condition after its last settled message. */
function deliveredCondition(last: Pick<Message, "status" | "conditions">) {
    // deliver after a sent message
    if (last.status === "sent") {
        return { status: "true" as const, reason: "Delivered", message: "" };
    }

    // fail with the failed message's Sent condition
    const sent = messageConditions.read(last, "Sent");

    return {
        status: "false" as const,
        reason: "Failing",
        message: present(sent, "a failed message's Sent condition").message,
    };
}

/** Refuse a secret header every webhook post sets itself. */
function requireHeader(authentication: WebhookAuthentication): void {
    if (
        authentication.kind === "header" &&
        RESERVED_HEADERS.has(authentication.name.toLowerCase())
    ) {
        throw MessageError.of(
            "HEADER_RESERVED",
            `every webhook post sets ${authentication.name} itself`,
        );
    }
}

/** Bind the installation as a consumer of an endpoint's secret, as the caller. */
async function consume(mutation: MethodContext, reference: ObjectReference): Promise<void> {
    const consumer = Caller.principal(mutation.self);
    await mutation.mutate(mutation.caller, [
        secret.call("bindConsumer", { ...secretTarget(reference), consumer }),
    ]);
}

/** Release the installation's use of a secret no endpoint of its scope names any longer. */
async function releaseUnused(mutation: MethodContext, reference: ObjectReference): Promise<void> {
    // skip a secret another endpoint still uses
    const key = ObjectReference.key(reference);
    const others = await mutation.read(endpoint).findMany({ where: { scope: mutation.scope } });
    if (others.some((other) => ObjectReference.key(other.secret) === key)) {
        return;
    }

    // release it as the caller, who may let software use the secret
    const consumer = Caller.principal(mutation.self);
    await mutation.mutate(mutation.caller, [
        secret.call("releaseConsumer", { ...secretTarget(reference), consumer }),
    ]);
}

/** Target the vault secret a reference names. */
function secretTarget(reference: ObjectReference) {
    return secret.target({ scope: reference.scope, id: secret.identifier(reference.id) });
}

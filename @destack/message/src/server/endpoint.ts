import type { Deriver } from "@destack/identity";
import { ObjectReference } from "@destack/sync";
import { Caller } from "@destack/access";
import { and, type DatabaseConnection, desc, eq, inArray, ne } from "@destack/db";
import {
    AuditCall,
    call,
    type Call,
    type ObjectReconciliation,
    ObjectWatch,
} from "@destack/object";
import { secret } from "@destack/vault/object";
import { aligned, Identifier, type JsonValue, present } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import {
    type Endpoint,
    endpoint,
    endpointConditions,
    type Message,
    message,
    messageAttempt,
    messageConditions,
    MessageKey,
} from "../object/index.ts";

/** The calls on one page of an endpoint's history: a hundred 1 KB calls post one 100 KB batch. */
const PAGE_CALLS = 100;

/** The record type of a batch of calls. */
const RECORD_TYPE = "call";

/** The object types whose controller calls deliver messages, which no endpoint receives. */
const DELIVERY_TYPES: ReadonlySet<string> = new Set([
    message.name,
    messageAttempt.name,
    endpoint.name,
]);

/** Implement endpoints: a controller delivers each enabled endpoint's calls as webhook messages. */
export function implementEndpoint(keys: Pick<Deriver, "derivePrivateKey">) {
    return endpoint.implement({
        methods: {
            create: async (mutation, next) => {
                // let the installation consume the secret, as its creator may let software use it
                const created = await next();
                await consume(mutation, created.secret);

                return created;
            },
            update: async (mutation, next) => {
                // let the installation consume a new secret, and release the replaced one once unused
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
            pending: { status: "enabled" },
            watches: [
                ObjectWatch.of(message.table, (row) => {
                    // wake the endpoint a sent message of it names
                    const source = ObjectReference.at("source", row);

                    return endpoint.is(source) && row.status !== "pending"
                        ? [{ id: source.id }]
                        : [];
                }),
                ObjectWatch.of(call.table, async (row, database) => {
                    // wake the enabled endpoints of the scope keeping a succeeded call
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
                            ),
                        );

                    return enabled.map((entry) => ({ id: entry.id }));
                }),
            ],
            reconcile: (reconciliation) =>
                deliver(reconciliation, aligned(reconciliation.rows, 0), keys),
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
    const items = page.events.map((event) => AuditCall.parse(event.data));

    // hand the subscribed calls to messages, each identified by its request, leaving those a repeated page already created
    const calls = items.filter((item) => isSubscribed(row, item));
    const { key } = await MessageKey.derive(keys);
    const described = await describeMessages(row, calls, key, reconciliation.now);
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

    // keep the endpoint's place past the page, and the calls after it
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

/** Report whether an endpoint receives a call: one of its events, and no message delivery. */
function isSubscribed(row: Endpoint, ended: AuditCall): boolean {
    return !isDelivery(ended) && (row.events === "all" || row.events.includes(ended.method));
}

/** Report whether a call is one of this package's controllers delivering endpoint messages. */
function isDelivery(ended: AuditCall): boolean {
    const { component, package: served } = ended.execution.context;
    const type = ended.method.slice(0, ended.method.lastIndexOf("."));

    return component !== undefined && served.id === message.package.id && DELIVERY_TYPES.has(type);
}

/** Describe the messages with a page's calls to an endpoint: an event per call, or one batch. */
async function describeMessages(
    row: Endpoint,
    calls: readonly AuditCall[],
    key: string,
    now: number,
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
                    ),
                    type: ended.method,
                    data: {
                        scope: execution.context.scope,
                        target: execution.target,
                        details: execution.details,
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
            ),
            type: RECORD_TYPE,
            data: [...calls],
        }),
    ];
}

/** Describe one message to an endpoint, sealed to its home's key and signed by its secret. */
async function describeMessage(
    row: Endpoint,
    key: string,
    now: number,
    sent: {
        readonly requestId: string;
        readonly type: string;
        readonly data: JsonValue;
    },
) {
    return {
        scope: row.scope,
        requestId: sent.requestId,
        id: Identifier.derive("message", sent.requestId),
        input: {
            to: { channel: "webhook" as const },
            dueAt: now,
            status: "pending" as const,
            ciphertext: await MessageKey.of(key).seal(
                { channel: "webhook", type: sent.type, format: row.format, data: sent.data },
                row.scope,
            ),
            ...ObjectReference.columns("source", endpoint.reference(row.scope, row.id)),
        },
    };
}

/** Read the last message to an endpoint its controller sent or gave up. */
async function lastSettled(
    database: DatabaseConnection,
    row: Endpoint,
): Promise<Pick<Message, "status" | "conditions"> | undefined> {
    const [last] = await database
        .select({ status: message.table.status, conditions: message.table.conditions })
        .from(message.table)
        .where(
            and(
                eq(message.table.sourceType, endpoint.name),
                eq(message.table.sourceId, row.id),
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

/** Bind the installation as a consumer of an endpoint's secret, as the caller. */
async function consume(mutation: Call, reference: ObjectReference): Promise<void> {
    const consumer = Caller.principal(mutation.self);
    await mutation.mutate(mutation.caller, [
        secret.call("bindConsumer", { ...secretTarget(reference), consumer }),
    ]);
}

/** Release the installation's use of a secret no endpoint of its scope names any longer. */
async function releaseUnused(mutation: Call, reference: ObjectReference): Promise<void> {
    // keep a secret another endpoint still names
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

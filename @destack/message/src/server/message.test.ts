import type { Call } from "@destack/sync";
import { AccessContext, Caller, Policy, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { call, JournalEntry, journal } from "@destack/object";
import { testEvents } from "@destack/object/test";
import { defineServer, ObjectStore } from "@destack/object/server";
import { defineDatabase, type DatabaseConnection, eq, TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Derivation } from "@destack/identity";
import type {} from "@destack/package/import-meta";
import { type Identifier, present, schema, Clock } from "@destack/schema";
import { type Controller, ControlLoop } from "@destack/runtime/control";
import { testCallKey } from "@destack/runtime/test";
import { ObjectReference, Scope } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { message, messageAttempt, messageConditions, MessageKey } from "../object/index.ts";
import type { MessageProvider, Outcome } from "../provider/index.ts";
import { pushProvider, Vapid } from "../push/index.ts";
import { messageTables } from "../stack/index.ts";
import type { MessageOptions } from "./message.ts";
import messagePackage from "../package.ts";
import { implementEndpoint } from "./endpoint.ts";
import { implementMessage } from "./message.ts";
import { RequestId } from "@destack/runtime/request";

/** The scope type the messages live in, as a space runs on a machine. */
const place = new Policy(import.meta.destack.package, {
    name: "place",
    permissions: {},
    scope: true,
});

/** The space whose server keeps the messages and owns their scope. */
const SPACE = principal.space.reference(
    Scope.universe.id,
    "space-019f5530-8000-7000-8000-0000000000e4",
);

/** The root secret the scope's message key derives from. */
const ROOT = Derivation.of(await Derivation.root(crypto.getRandomValues(new Uint8Array(32))));

/** A compact JWE inside stored JSON, as sealed content is kept. */
const COMPACT_JWE = /eyJ[\w-]+\.[\w-]*\.[\w-]+\.[\w-]+\.[\w-]+/u;

/** The text of the requested email, which only its provider sees. */
const TEXT = "title is too long";

/** The desktop client showing a scheduled message. */
const CLIENT = schema.identifier("client").parse("client-019f5530-8000-7000-8000-0000000000c1");

/** The machine serving the messages. */
const MACHINE = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-0000000000a1");

/** How refused sends retry here: three times at 10 ms. */
const RETRY = {
    initialInterval: 10,
    maximumInterval: 10,
    maximumAttempts: 3,
    backoffCoefficient: 1,
};

/** The longest a test waits for a message to settle, far above three retries at 10 ms. */
const SETTLE_MILLISECONDS = 5000;

/** A database keeping messages, as a machine keeps its spaces'. */
const messageDatabase = defineDatabase({
    name: "message",
    tables: [...messageTables, journal, ...call.tables],
});

/** Create a new message's identifier. */
function newId(): Identifier<"message"> {
    return message.generateId();
}

/** Run some controllers over a database until the test ends, failing it on a reported failure. */
function run(database: DatabaseConnection, controllers: readonly Controller[]): void {
    const stopping = new AbortController();
    const running = new ControlLoop(database, controllers, {
        clock: Clock.system,
        report: (_controller, _key, error) => {
            throw error;
        },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
}

/** Serve a scope's messages through an email provider answering in turn. */
async function startMessages(answers: readonly Outcome[]) {
    // keep the messages, sending each email through a provider answering in turn
    const { provider, sent, texts } = answerInTurn(answers);
    const { messages, storage, scope } = await openMessages({ email: provider });
    run(storage.database, messages.controllers());

    // request an email in the scope
    const id = message.generateId();
    await messages.mutate(AccessContext.of(Caller.of(SPACE), Date.now()), {
        id: RequestId.create(Date.now()),
        calls: [
            message.call("create", {
                ...message.target({ scope: scope.id, id }),
                to: { channel: "email", address: "dana@example.com" },
                ...ObjectReference.columns("source", scope),
                dueAt: Date.now(),
                status: "pending",
                ciphertext: await (
                    await MessageKey.derive(ROOT)
                ).seal({ channel: "email", subject: "Regressed", text: TEXT }, scope.id),
            }),
        ],
    });

    return {
        read: () => readSending(storage.database, id),
        until: (check: (sending: Sending) => boolean) =>
            TestDatabase.settle(
                storage.database,
                async () => {
                    const sending = await readSending(storage.database, id);

                    return sending !== undefined && check(sending);
                },
                SETTLE_MILLISECONDS,
            ),
        leaks: () => listLeaks(storage.database),
        callers: () => listCallers(storage.database),
        sent,
        texts,
    };
}

/** Keep a scope the space owns, and serve its messages through the given providers. */
async function openMessages(providers: MessageOptions["providers"]) {
    // serve the messages over a database of their own
    const storage = await TestDatabase.create("sqlite", messageDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const messages = new ObjectStore({
        servers: [
            defineServer(messagePackage, {
                objects: [
                    implementMessage({ providers, keys: ROOT, retry: RETRY }),
                    implementEndpoint(ROOT),
                ],
            }),
        ],
        chain: async () => undefined,
        clock: Clock.system,
        principal: SPACE,
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        policies: [place],
        origin: { package: message.package },
        machine: MACHINE,
    });

    // keep the scope, owned by the space
    const scope = {
        ...place.reference(Scope.universe.id, "place-019f5530-8000-7000-8000-0000000000e3"),
    };
    const fixture = new AccessFixture(storage.database);
    await fixture.replicateScope(scope);
    await fixture.bindOwner(scope, SPACE);

    return { messages, storage, scope };
}

/** Provide email through answers in turn, keeping the identifier and text of each send. */
function answerInTurn(answers: readonly Outcome[]) {
    const sent: string[] = [];
    const texts: string[] = [];
    const provider: MessageProvider = {
        send: async (sending) => {
            sent.push(sending.id);
            texts.push(sending.content.channel === "email" ? sending.content.text : "");

            return present(answers[sent.length - 1], "an answer for each send");
        },
    };

    return { provider, sent, texts };
}

/** A message's sending as a test reads it. */
type Sending = NonNullable<Awaited<ReturnType<typeof readSending>>>;

/** Read a message's sending: its status, its erasure, its attempts and its Sent condition. */
async function readSending(database: DatabaseConnection, id: Identifier<"message">) {
    // read the message's status and erasure
    const [row] = await database
        .select({
            status: message.table.status,
            reason: message.table.reason,
            ciphertext: message.table.ciphertext,
            conditions: message.table.conditions,
        })
        .from(message.table)
        .where(eq(message.table.id, id));
    const attempts = await database
        .select({ outcome: messageAttempt.table.outcome })
        .from(messageAttempt.table)
        .where(eq(messageAttempt.table.parentId, id))
        .orderBy(messageAttempt.table.createdAt, messageAttempt.table.id);

    // read the Sent condition's status and reason
    const sent = row && messageConditions.read(row, "Sent");

    return (
        row && {
            status: row.status,
            reason: row.reason,
            isErased: row.ciphertext === null,
            attempts: attempts.map(({ outcome }) => outcome.kind),
            sent: sent && { status: sent.status, reason: sent.reason },
        }
    );
}

/** List each journaled call's method with the principal and component making it, in order. */
async function listCallers(database: DatabaseConnection) {
    const rows = await database.select().from(journal).orderBy(journal.startedAt, journal.position);

    return rows.map(({ call: recorded }) => {
        const entry = JournalEntry.parse(recorded);

        return [entry.method, entry.execution.context.caller, entry.execution.context.component];
    });
}

/** List the tables whose rows have the content's text, or its ciphertext as a compact JWE. */
async function listLeaks(database: DatabaseConnection): Promise<string[]> {
    const leaking: string[] = [];
    for (const table of messageTables) {
        const text = JSON.stringify(await database.select().from(table));
        if (text.includes(TEXT)) {
            leaking.push(`${table[TABLE].name}: text`);
        }
        if (COMPACT_JWE.test(text)) {
            leaking.push(`${table[TABLE].name}: ciphertext`);
        }
    }

    return leaking;
}

test("send a requested message through its channel's provider, retrying a temporary refusal, as the space and its message controller", async () => {
    const { read, until, sent, callers } = await startMessages([
        { kind: "retry", error: { code: "Throttling", message: "slow down" } },
        { kind: "sent" },
    ]);

    // send it on the second attempt
    await until((sending) => sending.status !== "pending");
    expect(await read()).toEqual({
        status: "sent",
        reason: null,
        isErased: true,
        attempts: ["retry", "sent"],
        sent: { status: "true", reason: "Sent" },
    });

    // record the request and each attempt and observation as the space, the controller's calls under its name
    const space = { subject: SPACE };
    expect([sent.length, await callers()]).toEqual([
        2,
        [
            ["message.create", space, undefined],
            ["attempt.create", space, "message"],
            ["message.observe", space, "message"],
            ["attempt.create", space, "message"],
            ["message.observe", space, "message"],
        ],
    ]);
});

test("fail a message its provider refuses for good, or that the retries exhaust", async () => {
    const refused = await startMessages([
        {
            kind: "failed",
            error: { code: "MessageRejected", message: "address is not verified" },
        },
    ]);
    const exhausted = await startMessages(
        Array.from({ length: 3 }, () => ({
            kind: "retry" as const,
            error: { code: "500", message: "down" },
        })),
    );

    // fail at once on a refusal for good, and after the policy's attempts on temporary refusals
    await refused.until((sending) => sending.status !== "pending");
    await exhausted.until((sending) => sending.status !== "pending");
    expect(await refused.read()).toEqual({
        status: "failed",
        reason: "MessageRejected",
        isErased: true,
        attempts: ["failed"],
        sent: { status: "false", reason: "MessageRejected" },
    });
    expect(await exhausted.read()).toEqual({
        status: "failed",
        reason: "500",
        isErased: true,
        attempts: ["retry", "retry", "retry"],
        sent: { status: "false", reason: "500" },
    });
});

test("open a message's content only for its provider, keeping it out of the journal and the change log, and drop it as the message settles", async () => {
    const { read, until, texts, leaks } = await startMessages([{ kind: "sent" }]);

    // send the plaintext while no stored row has it
    await until((sending) => sending.status !== "pending");
    expect(await read()).toEqual({
        status: "sent",
        reason: null,
        isErased: true,
        attempts: ["sent"],
        sent: { status: "true", reason: "Sent" },
    });
    expect({ texts, leaking: await leaks() }).toEqual({
        texts: [TEXT],
        leaking: [],
    });
});

test("drop a message's sealed content as it settles, leaving no copy in any row or journal entry and keeping its attempts", async () => {
    const sent = await startMessages([{ kind: "sent" }]);
    const failed = await startMessages([
        { kind: "failed", error: { code: "MessageRejected", message: "unverified" } },
    ]);

    // drop the content of a sent and a failed message
    await sent.until((sending) => sending.isErased);
    await failed.until((sending) => sending.isErased);
    expect(await sent.read()).toEqual({
        status: "sent",
        reason: null,
        isErased: true,
        attempts: ["sent"],
        sent: { status: "true", reason: "Sent" },
    });
    expect(await failed.read()).toEqual({
        status: "failed",
        reason: "MessageRejected",
        isErased: true,
        attempts: ["failed"],
        sent: { status: "false", reason: "MessageRejected" },
    });

    // keep no copy of the content in any row or journal entry
    expect([await sent.leaks(), await failed.leaks()]).toEqual([[], []]);
});

test("schedule messages, then send, defer or skip each while it waits, showing a desktop one at once until its client receives it", async () => {
    const { messages, storage, scope } = await openMessages({});
    const now = Date.now();
    const mutate = (calls: readonly Call[]) =>
        messages.mutate(AccessContext.of(Caller.of(SPACE), Date.now()), {
            id: RequestId.create(now),
            calls: [...calls],
        });
    const sealed = await (
        await MessageKey.derive(ROOT)
    ).seal({ channel: "email", subject: "Mentioned", text: TEXT }, scope.id);

    // schedule an email, a push, a second email and a desktop message
    const email = { channel: "email" as const, address: "dana@example.com" };
    const push = {
        channel: "push" as const,
        url: "https://push.example/endpoint",
        keys: { p256dh: "key", auth: "secret" },
    };
    const desktop = { channel: "desktop" as const, client: CLIENT };
    const [first, second, third, fourth] = [newId(), newId(), newId(), newId()];
    const destinations = [
        [first, email],
        [second, push],
        [third, email],
        [fourth, desktop],
    ] as const;
    await mutate(
        destinations.map(([id, to]) =>
            message.call("create", {
                ...message.target({ scope: scope.id, id }),
                to,
                ...ObjectReference.columns("source", scope),
                dueAt: now,
                status: "scheduled",
            }),
        ),
    );

    // send the first email, move the push into a summary, skip the second email and show the desktop one
    await mutate([
        message.call("send", { scope: scope.id, id: first, to: email, ciphertext: sealed }),
        message.call("defer", {
            scope: scope.id,
            id: second,
            dueAt: now + 60_000,
            isSummarized: true,
        }),
        message.call("skip", { scope: scope.id, id: third, reason: "read" }),
        message.call("send", { scope: scope.id, id: fourth, to: desktop }),
    ]);

    // mark the desktop one delivered once its client receives it, twice alike
    await mutate([message.call("receive", { scope: scope.id, id: fourth })]);
    await mutate([message.call("receive", { scope: scope.id, id: fourth })]);

    // refuse what each state forbids: deciding again, a receipt off a desktop, a send without content
    const refused = await Promise.all([
        refusal(mutate([message.call("skip", { scope: scope.id, id: first, reason: "read" })])),
        refusal(mutate([message.call("receive", { scope: scope.id, id: first })])),
        refusal(mutate([message.call("send", { scope: scope.id, id: second, to: push })])),
    ]);
    expect(refused).toEqual([
        ["CONFLICT", `message ${first} is pending and cannot skip`],
        ["CONFLICT", `message ${first} is not a sent desktop message`],
        ["BAD_REQUEST", `message ${second} on push has no content`],
    ]);

    // keep each message's state
    const rows = await storage.database
        .select({
            id: message.table.id,
            status: message.table.status,
            reason: message.table.reason,
            dueAt: message.table.dueAt,
            isSummarized: message.table.isSummarized,
            hasContent: message.table.ciphertext,
        })
        .from(message.table);
    const states = new Map(
        rows.map((row) => [row.id, { ...row, hasContent: row.hasContent !== null }]),
    );
    expect([first, second, third, fourth].map((id) => states.get(id))).toEqual([
        {
            id: first,
            status: "pending",
            reason: null,
            dueAt: now,
            isSummarized: false,
            hasContent: true,
        },
        {
            id: second,
            status: "scheduled",
            reason: null,
            dueAt: now + 60_000,
            isSummarized: true,
            hasContent: false,
        },
        {
            id: third,
            status: "skipped",
            reason: "read",
            dueAt: now,
            isSummarized: false,
            hasContent: false,
        },
        {
            id: fourth,
            status: "delivered",
            reason: null,
            dueAt: now,
            isSummarized: false,
            hasContent: false,
        },
    ]);
});

test("read the key a scope's pushes are signed with, refusing it on a machine sending no pushes", async () => {
    // serve the scope's messages with a push provider, and again with none
    const vapid = await Vapid.derive(ROOT, "https://destack.app");
    const pushing = await openMessages({ push: pushProvider(async () => vapid) });
    const silent = await openMessages({});
    const read = (opened: typeof pushing) =>
        opened.messages.query(
            AccessContext.of(Caller.of(SPACE), Date.now()),
            message.call("serverKey", { scope: opened.scope.id }),
        );

    // answer the provider's VAPID public key, and refuse where no provider sends pushes
    expect([await read(pushing), await refusal(read(silent))]).toEqual([
        { key: await vapid.publicKey() },
        ["NOT_FOUND", "this machine does not send pushes"],
    ]);
});

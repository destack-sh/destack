import { EventFixture } from "@destack/event/test";
import { call } from "@destack/audit";
import { Policy, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { journal } from "@destack/audit/stack";
import { Journal } from "@destack/audit/server";
import { ObjectServer } from "@destack/object/server";
import { defineDatabase, type DatabaseConnection, eq, TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Derivation } from "@destack/identity";
import type {} from "@destack/package/import-meta";
import { type Duration, type Identifier, present, schema } from "@destack/schema";
import { type Controller, ControlLoop } from "@destack/service/control";
import { testCallKey } from "@destack/service/test";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { message, messageAttempt, messageConditions, MessageKey } from "../object/index.ts";
import type { MessageProvider, Outcome } from "../provider/index.ts";
import { messageTables } from "../stack/index.ts";
import { implementMessages } from "./server.ts";

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

/** The machine serving the messages. */
const MACHINE = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-0000000000a1");

/** How refused sends retry here: three times at 10 ms. */
const RETRY = {
    initialInterval: 10,
    maximumInterval: 10,
    maximumAttempts: 3,
    backoffCoefficient: 1,
};

/** A database keeping messages, as a machine keeps its spaces'. */
const messageDatabase = defineDatabase({ name: "message", tables: [...messageTables, journal] });

/** Run some controllers over a database until the test ends, failing it on a reported failure. */
function run(database: DatabaseConnection, controllers: readonly Controller[]): void {
    const stopping = new AbortController();
    const running = new ControlLoop(database, controllers, {
        report: (_controller, _key, error) => {
            throw error;
        },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
}

/** Serve a scope's messages, sending them through an email provider answering in turn, and keeping their content for a retention. */
async function serveMessages(answers: readonly Outcome[], retention: Duration = { minutes: 1 }) {
    // keep the messages, sending each email through a provider answering in turn
    const { provider, sent, texts } = answerInTurn(answers);
    const storage = await TestDatabase.create("sqlite", messageDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const { server: messages } = ObjectServer.compose(
        [
            implementMessages({
                providers: { email: provider },
                deriver: async () => ROOT,
                events: (await EventFixture.open("sqlite", [call])).store,
                retry: RETRY,
                retention,
            }),
        ],
        {
            principal: SPACE,
            database: storage.database,
            callKey: testCallKey,
            policies: [place],
            origin: { package: message.package, service: "message" },
            machine: MACHINE,
        },
    );
    run(storage.database, messages.controllers());

    // request an email in a scope
    const scope = {
        ...place.reference(Scope.universe.id, "place-019f5530-8000-7000-8000-0000000000e3"),
    };
    const fixture = new AccessFixture(storage.database);
    await fixture.copyScope(scope);
    await fixture.copyOwner(scope, SPACE);
    const id = schema.identifier("message").parse(message.generateId());
    await messages.execute(
        SPACE,
        message,
        "create",
        [
            {
                scope: scope.id,
                id,
                input: {
                    to: { channel: "email", address: "dana@example.com" },
                    source: scope,
                    status: "pending",
                    ciphertext: await (
                        await MessageKey.derive(ROOT)
                    ).seal({ channel: "email", subject: "Regressed", text: TEXT }, scope.id),
                },
            },
        ],
        Date.now(),
    );

    return {
        read: () => readSending(storage.database, id),
        leaks: () => listLeaks(storage.database),
        callers: () => listCallers(storage.database),
        sent,
        texts,
    };
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

/** Read a message's sending: its status, its erasure, its attempts and its Sent condition. */
async function readSending(database: DatabaseConnection, id: Identifier<"message">) {
    // read the message's status and erasure
    const [row] = await database
        .select({
            status: message.table.status,
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
            isErased: row.ciphertext === null,
            attempts: attempts.map(({ outcome }) => outcome.kind),
            sent: sent && { status: sent.status, reason: sent.reason },
        }
    );
}

/** List each journaled call's method with the principal and component making it, in order. */
async function listCallers(database: DatabaseConnection) {
    const calls = await new Journal(database, testCallKey).read({ limit: 100 });

    return calls.map((recorded) => [
        recorded.method,
        recorded.execution.context.caller,
        recorded.execution.context.component,
    ]);
}

/** List the tables whose stored rows hold the content's text, or its ciphertext as a compact JWE. */
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
    const { read, sent, callers } = await serveMessages([
        { kind: "retry", error: { code: "Throttling", message: "slow down" } },
        { kind: "sent" },
    ]);

    // send it on the second attempt
    await expect.poll(read, { timeout: 10_000 }).toEqual({
        status: "sent",
        isErased: false,
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
    const refused = await serveMessages([
        {
            kind: "failed",
            error: { code: "MessageRejected", message: "address is not verified" },
        },
    ]);
    const exhausted = await serveMessages(
        Array.from({ length: 3 }, () => ({
            kind: "retry" as const,
            error: { code: "500", message: "down" },
        })),
    );

    // fail at once on a refusal for good, and after the policy's attempts on temporary refusals
    await expect.poll(refused.read, { timeout: 10_000 }).toEqual({
        status: "failed",
        isErased: false,
        attempts: ["failed"],
        sent: { status: "false", reason: "MessageRejected" },
    });
    await expect.poll(exhausted.read, { timeout: 10_000 }).toEqual({
        status: "failed",
        isErased: false,
        attempts: ["retry", "retry", "retry"],
        sent: { status: "false", reason: "500" },
    });
});

test("keep a message's content sealed in its row alone, out of the journal and the change log, and open it only for its provider", async () => {
    const { read, texts, leaks } = await serveMessages([{ kind: "sent" }]);

    // send the plaintext while no stored row holds it
    await expect.poll(read, { timeout: 10_000 }).toEqual({
        status: "sent",
        isErased: false,
        attempts: ["sent"],
        sent: { status: "true", reason: "Sent" },
    });
    expect({ texts, leaking: await leaks() }).toEqual({
        texts: [TEXT],
        leaking: [`${message.table[TABLE].name}: ciphertext`],
    });
});

test("erase a settled message's sealed content once its retention passes, leaving no copy in any row or journal entry and keeping its attempts", async () => {
    const sent = await serveMessages([{ kind: "sent" }], { milliseconds: 0 });
    const failed = await serveMessages(
        [{ kind: "failed", error: { code: "MessageRejected", message: "unverified" } }],
        { milliseconds: 0 },
    );

    // erase the content of a sent and a failed message
    await expect.poll(sent.read, { timeout: 10_000 }).toEqual({
        status: "sent",
        isErased: true,
        attempts: ["sent"],
        sent: { status: "true", reason: "Sent" },
    });
    await expect.poll(failed.read, { timeout: 10_000 }).toEqual({
        status: "failed",
        isErased: true,
        attempts: ["failed"],
        sent: { status: "false", reason: "MessageRejected" },
    });

    // keep no copy of the content in any row or journal entry
    expect([await sent.leaks(), await failed.leaks()]).toEqual([[], []]);
});

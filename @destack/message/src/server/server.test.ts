import { defineDatabase, type DatabaseConnection, eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { ObjectServer } from "@destack/object/server";
import { present, schema } from "@destack/schema";
import { type Controller, ControlLoop } from "@destack/service/control";
import { testCallKey } from "@destack/service/test";
import { serveObjects } from "@destack/space/server";
import { spaceCopies, spaceTables } from "@destack/space/stack";
import { ids, openDirectory, SpaceFixture } from "@destack/space/test";
import { expect, onTestFinished, test } from "@destack/test";
import { message, messageAttempt } from "../object/index.ts";
import type { MessageProvider, Outcome } from "../provider/index.ts";
import { messageDatabase } from "../stack/index.ts";
import { implementMessages } from "./server.ts";

/** A cell's space database copying the messages the message service keeps. */
const cellDatabase = defineDatabase({
    name: "space",
    tables: spaceTables,
    copies: [...spaceCopies, message.table],
});

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

/** Serve a space copying messages, and the message service sending them through an email provider answering in turn. */
async function serveMessages(answers: readonly Outcome[]) {
    // keep the messages, sending each email through a provider answering in turn
    const sent: string[] = [];
    const provider: MessageProvider = {
        channel: "email",
        send: async (sending) => {
            sent.push(sending.id);

            return present(answers[sent.length - 1], "an answer for each send");
        },
    };
    const storage = await TestDatabase.create("sqlite", messageDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    let spaces: ObjectServer | undefined;
    const messages = implementMessages({
        database: storage.database,
        callKey: testCallKey,
        providers: [provider],
        cell: ids.region,
        spaces: {
            stream: (subscription, signal) =>
                present(spaces, "the space service").uplink.stream(subscription, signal),
            receive: (mutation) => present(spaces, "the space service").uplink.receive(mutation),
        },
        retry: {
            initialInterval: 10,
            maximumInterval: 10,
            maximumAttempts: 3,
            backoffCoefficient: 1,
        },
    });

    // serve a space copying them, running both sides' controllers
    const fixture = await SpaceFixture.open({ database: cellDatabase });
    const space = serveObjects(
        await fixture.options({ ...(await openDirectory()), sources: [messages.objects] }),
    );
    spaces = space;
    run(
        fixture.database,
        space.controllers().filter((controller) => ["sends", "replica"].includes(controller.name)),
    );
    run(storage.database, present(messages.controllers, "the message service's controllers"));

    // request an email through a call sent to the message service
    const id = schema.identifier("message").parse(message.generateId());
    await fixture.database.transaction((transaction) =>
        space.change(
            { database: transaction, scope: ids.space, now: Date.now() },
            message,
            "create",
            {
                id,
                to: { channel: "email", address: "dana@example.com" },
                content: { channel: "email", subject: "Regressed", text: "title is too long" },
            },
        ),
    );

    // read the space's copy of the message's sending
    const copied = async () => {
        const [row] = await fixture.database
            .select({
                sentAt: message.table.sentAt,
                failedAt: message.table.failedAt,
                conditions: message.table.conditions,
            })
            .from(message.table)
            .where(eq(message.table.id, id));

        return (
            row && {
                isSent: row.sentAt !== null,
                isFailed: row.failedAt !== null,
                attempts: (
                    await storage.database
                        .select({ outcome: messageAttempt.table.outcome })
                        .from(messageAttempt.table)
                        .where(eq(messageAttempt.table.parentId, id))
                        .orderBy(messageAttempt.table.createdAt, messageAttempt.table.id)
                ).map(({ outcome }) => outcome),
                sent: row.conditions["Sent"] && {
                    status: row.conditions["Sent"].status,
                    reason: row.conditions["Sent"].reason,
                },
            }
        );
    };

    return { copied, sent };
}

test("send a message a space requests through its channel's provider, retrying a refusal for now", async () => {
    const { copied, sent } = await serveMessages([
        { outcome: "retry", error: { code: "Throttling", message: "slow down" } },
        { outcome: "sent" },
    ]);

    // send it on the second attempt and copy the sending back to the space
    await expect.poll(copied, { timeout: 10_000 }).toEqual({
        isSent: true,
        isFailed: false,
        attempts: ["retry", "sent"],
        sent: { status: "true", reason: "Sent" },
    });
    expect(sent.length).toBe(2);
});

test("fail a message its provider refuses for good, or that the retries exhaust", async () => {
    const refused = await serveMessages([
        {
            outcome: "failed",
            error: { code: "MessageRejected", message: "address is not verified" },
        },
    ]);
    const exhausted = await serveMessages(
        Array.from({ length: 3 }, () => ({
            outcome: "retry" as const,
            error: { code: "500", message: "down" },
        })),
    );

    // fail at once on a refusal for good, and after the policy's attempts on refusals for now
    await expect.poll(refused.copied, { timeout: 10_000 }).toEqual({
        isSent: false,
        isFailed: true,
        attempts: ["failed"],
        sent: { status: "false", reason: "MessageRejected" },
    });
    await expect.poll(exhausted.copied, { timeout: 10_000 }).toEqual({
        isSent: false,
        isFailed: true,
        attempts: ["retry", "retry", "retry"],
        sent: { status: "false", reason: "500" },
    });
});

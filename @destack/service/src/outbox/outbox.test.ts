import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { Outbox, outbox, type Destination } from "./outbox.ts";

/** A message two destinations receive. */
const Message = schema.object({ text: schema.string(), note: schema.string().optional() });
/** A message two destinations receive. */
type Message = schema.Infer<typeof Message>;

/** Collect the batches a destination accepts. */
function collecting(name: string, batch: number): Destination<Message> & { batches: Message[][] } {
    const batches: Message[][] = [];

    return {
        name,
        message: Message,
        batch,
        batches,
        accept: async (messages) => batches.push([...messages]),
    };
}

for (const dialect of TEST_DIALECTS) {
    test(`deliver each destination's messages in order and in batches, once per key (${dialect})`, async () => {
        const storage = await TestDatabase.create(dialect, [outbox], { isMigrated: true });
        onTestFinished(() => storage.close());
        const sender = new Outbox(storage.database);
        const mail = collecting("mail", 2);
        const inbox = collecting("inbox", 2);

        // append three mails and an inbox message
        await storage.database.transaction(async (transaction) => {
            for (const text of ["first", "second", "third"]) {
                await sender.append(mail, `mail-${text}`, { text }, transaction);
            }
        });
        await sender.append(inbox, "inbox-first", { text: "first", note: undefined });
        await sender.append(mail, "mail-first", { text: "first" });

        // deliver the mails in batches of two
        const delivered = [
            await sender.deliver(mail),
            await sender.deliver(mail),
            await sender.deliver(mail),
        ];
        expect([delivered, mail.batches, await sender.read(inbox, 10)]).toEqual([
            [2, 1, 0],
            [[{ text: "first" }, { text: "second" }], [{ text: "third" }]],
            [{ text: "first" }],
        ]);
    });
}

test("refuse a held key with other contents, and a transaction on another database", async () => {
    const storage = await TestDatabase.create("sqlite", [outbox], { isMigrated: true });
    const other = await TestDatabase.create("sqlite", [outbox], { isMigrated: true });
    onTestFinished(() => storage.close());
    onTestFinished(() => other.close());
    const sender = new Outbox(storage.database);
    const mail = collecting("mail", 2);

    // hold a key, then append other contents under it
    await sender.append(mail, "mail-first", { text: "first" });
    await expect(sender.append(mail, "mail-first", { text: "other" })).rejects.toMatchObject({
        code: "CONFLICT",
        message: "mail message mail-first has conflicting contents",
    });

    // append inside a foreign transaction
    await expect(
        other.database.transaction((transaction) =>
            sender.append(mail, "mail-second", { text: "second" }, transaction),
        ),
    ).rejects.toThrow(new TypeError("outbox appends need a transaction on the outbox's database"));
});

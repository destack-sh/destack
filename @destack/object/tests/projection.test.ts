import { expect, onTestFinished, test } from "@destack/test";
import { ControlLoop } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { testCallKey } from "@destack/service/test";
import type { Installation } from "@destack/service/workload";
import { asc, defineDatabase, eq, type LogPosition } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";
import { Feed, Replica, replica, Subject } from "@destack/sync";
import { none, principal } from "@destack/access";
import { v7 } from "uuid";
import { defineObject, field, type ObjectType } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { objectDatabase } from "./schema.ts";
import { openSpace, space } from "./fixture/space.ts";

/** The space with the memos. */
const MEMOS = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** The recipient's home, which keeps a receipt of each memo they receive. */
const HOME = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** A memo one person receives. */
const memo = defineObject({
    name: "memo",
    plural: "memos",
    scope: space,
    fields: { recipient: field.subject(), title: field.string() },
    permissions: { read: none() },
    methods: (method) => ({ get: method.get("read") }),
    projections: (): readonly ObjectType[] => [receipt],
});

/** A person, whose home keeps the receipts of the memos they receive. */
const person = defineObject({
    name: "person",
    plural: "people",
    scope: "universe",
    isScope: true,
    fields: { home: field.string().optional() },
    permissions: { read: none() },
});

/** An installation keeping memos for a person. */
const sender = defineObject({
    name: "sender",
    plural: "senders",
    scope: person,
    inherited: {},
    fields: { source: field.string(), installation: field.string() },
    permissions: { read: none() },
});

/** A recipient's receipt of a memo: its own object with its own note and seen time, keeping the memo's title. */
const receipt = defineObject({
    name: "receipt",
    plural: "receipts",
    scope: space,
    fields: {
        source: field.reference(memo, { qualified: true }),
        title: field.string(),
        note: field.string().default(""),
        seenAt: field.time().optional(),
    },
    projected: {
        from: memo,
        to: "recipient",
        source: "source",
        fields: { title: "title" },
        clears: ["seenAt"],
        residence: { user: person, address: sender },
    },
    permissions: { read: none() },
    methods: (method) => ({ get: method.get("read") }),
});

/** Sort projected rows by their title. */
function byTitle(rows: unknown[][]): unknown[][] {
    return rows.toSorted((left, right) => String(left[1]).localeCompare(String(right[1])));
}

test.for(TEST_DIALECTS)(
    "project memos into receipts of their own in the recipient's home, keeping their titles current and their notes, clearing a changed one's seen time, and retracting a deleted and a left-out memo's receipt, on %s",
    async (dialect) => {
        // keep the memos and the receipts in one database
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({
                name: "main",
                tables: [...objectDatabase.tables, ...memo.tables, ...receipt.tables],
            }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        const database = storage.database;
        await openSpace(database, MEMOS);
        await openSpace(database, HOME);
        const recipient = Subject.key(principal.user.reference("universe", "user-1"));
        const create = async (title: string) => {
            const id = schema.identifier("memo").parse(`memo-${v7()}`);
            await database
                .insert(memo.table)
                .values({ id, scope: MEMOS, recipient, title, createdAt: 1, updatedAt: 1 });

            return id;
        };
        const first = await create("First");
        const second = await create("Second");
        const extra = await create("Extra");

        // follow the memos, projecting them into receipts in the home
        const feed = new Feed(database, [memo.table, replica]);
        const receipts = new Replica({
            name: "receipts",
            scope: MEMOS,
            tables: [],
            isRelayed: false,
            projectors: [present(receipt.projector(HOME), "the receipt's projector")],
        });
        const follow = (from: (after: LogPosition | undefined) => LogPosition | undefined) => {
            const controller = new AbortController();
            const following = receipts.follow(
                database,
                ({ after }, signal) =>
                    feed.subscribe(
                        { ...receipts.queries, memo: { table: memo.table, scopes: [MEMOS] } },
                        from(after),
                        signal,
                    ),
                controller.signal,
            );

            return async () => {
                controller.abort();
                await following;
            };
        };
        const reach = async () =>
            expect(
                await Replica.reach(
                    database,
                    MEMOS,
                    await database.log.position(),
                    AbortSignal.timeout(5000),
                ),
            ).toBe(true);
        const projected = async () =>
            (
                await database
                    .select()
                    .from(receipt.table)
                    .orderBy(asc(receipt.table.createdAt), asc(receipt.table.id))
            ).map((row) => [
                row.source.id,
                row.title,
                row.note,
                row.seenAt !== null,
                row.scope === HOME && row.id !== row.source.id,
                row.revision,
            ]);

        // project the memos under identifiers of their own, then keep a retitle's title and note while clearing its seen time in a new revision
        const stop = follow((after) => after);
        await reach();
        await database.update(receipt.table).set({ note: "Noted", seenAt: 5 });
        await database.update(memo.table).set({ title: "Renamed" }).where(eq(memo.table.id, first));
        await reach();
        expect(byTitle(await projected())).toEqual([
            [extra, "Extra", "Noted", true, true, 1],
            [first, "Renamed", "Noted", false, true, 2],
            [second, "Second", "Noted", true, true, 1],
        ]);

        // retract a deleted memo's receipt
        await database.delete(memo.table).where(eq(memo.table.id, second));
        await reach();
        expect(byTitle(await projected())).toEqual([
            [extra, "Extra", "Noted", true, true, 1],
            [first, "Renamed", "Noted", false, true, 2],
        ]);
        await stop();

        // project a memo created while stopped, retract the one a later snapshot leaves out, and keep an unchanged one's seen time
        await database
            .update(receipt.table)
            .set({ seenAt: 6 })
            .where(eq(receipt.table.title, "Renamed"));
        await database.delete(memo.table).where(eq(memo.table.id, extra));
        const third = await create("Third");
        const snapshot = follow(() => undefined);
        await reach();
        expect(byTitle(await projected())).toEqual([
            [first, "Renamed", "Noted", true, true, 2],
            [third, "Third", "", false, true, 1],
        ]);
        await snapshot();
    },
);

test("settle the address of a recipient without a user, and record it again once a row naming them changes", async () => {
    // keep the memos of a recipient the directory knows no user of
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...objectDatabase.tables, ...memo.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, MEMOS);
    const recipient = Subject.key(principal.user.reference("universe", "user-gone"));
    const create = async (title: string) => {
        const id = schema.identifier("memo").parse(`memo-${v7()}`);
        await database
            .insert(memo.table)
            .values({ id, scope: MEMOS, recipient, title, createdAt: 1, updatedAt: 1 });
    };
    await create("First");

    // record addresses through a directory refusing the unknown recipient
    const recorded: string[] = [];
    const waiting = new Map<number, () => void>();
    const installation: Installation = {
        id: "installation-01996ab0-0000-7000-8000-000000000003",
        scope: MEMOS,
        publisher: {
            stream: () => {
                throw new TypeError("the fixture streams no copies");
            },
        },
        reach: () => {
            throw new TypeError("the fixture reaches no installation");
        },
        directory: {
            home: async () => undefined,
            address: async (address) => {
                recorded.push(address);
                waiting.get(recorded.length)?.();
                throw new ServiceError("NOT_FOUND", { message: `no user ${address}` });
            },
        },
    };
    const recordedTimes = (count: number) =>
        new Promise<void>((resolve) => {
            waiting.set(count, resolve);
            if (recorded.length >= count) {
                resolve();
            }
        });
    const server = new ObjectServer({
        objects: { memo },
        database,
        callKey: testCallKey,
        origin: { package: memo.package, service: "test" },
        installation,
    });
    const failures: unknown[] = [];
    const loop = new ControlLoop(
        database,
        [present(server.source.addresses(), "the address controller")],
        { report: (_controller, _key, error) => failures.push(error) },
    );
    const controller = new AbortController();
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });

    // settle the recipient without retrying, then record it again for a new memo naming them
    await recordedTimes(1);
    await loop.idle();
    const settled = [...recorded];
    await create("Second");
    await recordedTimes(2);
    await loop.idle();
    expect([settled, recorded, failures]).toEqual([[recipient], [recipient, recipient], []]);
});

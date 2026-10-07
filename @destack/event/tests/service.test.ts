import { accessTables, Authorization, Authorizer, none, Policy, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineTable, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { LocalKeyring } from "@destack/identity";
import { MemoryBucket } from "@destack/bucket/test";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { createClient } from "@destack/service/client";
import { isServiceError } from "@destack/service/error";
import { Health } from "@destack/service/health";
import { Outbox, outbox } from "@destack/service/outbox";
import { Server, type ServiceContext } from "@destack/service/server";
import { expect, onTestFinished, test } from "@destack/test";
import { defineEventKind, type EventKind } from "../src/kind/index.ts";
import { DatabasePersonalKeyring } from "../src/personal/index.ts";
import { implementEvents, type UnmaskedRead } from "../src/server/index.ts";
import { eventService } from "../src/service/index.ts";
import { eventTables } from "../src/stack/index.ts";
import { EventStore } from "../src/store/index.ts";

/** The package declaring the test's policies and kind. */
const meters = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000021"),
    name: "@example/meters",
    version: "2026.9.0",
};

/** The accounts whose readings the test reads, each the scope of its own. */
const account = new Policy(meters, {
    name: "account",
    relations: {},
    permissions: { share: none() },
    grantedBy: "share",
    scope: true,
});

/** The readings of an account, read and shown with their personal values through roles granted on it. */
const reading = new Policy(meters, {
    name: "reading",
    relations: {},
    permissions: { read: none(), unmask: none() },
});

/** The table mapping accounts to their scopes. */
const accountRecord = defineTable("account_record", {
    id: text("id").primaryKey().notNull(),
    scope: text("scope").notNull(),
});

/** Readings of an account's meters, the person who took one sealing their note. */
const meterReading = defineEventKind(
    {
        name: "reading",
        description: "A meter's reading in an account.",
        keys: schema.object({ meter: schema.string(), person: schema.string() }),
        data: schema.object({ note: schema.sensitive(schema.string(), "personal") }),
        delivery: "at-most-once",
        policy: { flush: { maxAge: 3_600_000, maxRows: 100 }, retention: 86_400_000 },
        subject: "person",
        access: { read: reading.permission("read"), unmask: reading.permission("unmask") },
    },
    { package: meters },
);

/** The account the readings belong to. */
const ACCOUNT = "account-01995da9-7223-7000-8000-000000000001";

/** Serve an account's readings, granting reader read and alice unmask. */
async function serve(admit?: (context: ServiceContext, kind: EventKind) => Promise<void>) {
    // keep the readings beside the access rows deciding who reads them
    const tables = [...eventTables([meterReading]), outbox, accountRecord, ...accessTables];
    const created = await TestDatabase.create("sqlite", tables);
    onTestFinished(() => created.close());
    const { database } = created;
    await database.migrate(tables);
    const bucket = new MemoryBucket();
    const store = new EventStore({
        database,
        kinds: [meterReading],
        files: () => bucket,
        personal: new DatabasePersonalKeyring(
            database,
            await LocalKeyring.read(LocalKeyring.generate()),
        ),
        outbox: new Outbox(database),
    });

    // let the owner grant reader read, and alice read and unmask, on the account
    const authorizer = new Authorizer(
        [reading, account],
        [
            {
                policy: account,
                table: accountRecord,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: {},
            },
        ],
    );
    const owner = principal.user.reference("universe", "owner");
    const asOwner = new Authorization(authorizer, database, () => ({
        subjects: [owner],
        now: Date.now(),
        attributes: {},
    }));
    await new AccessFixture(database).copyScope(owner);
    await database.insert(accountRecord).values({ id: ACCOUNT, scope: "owner" });
    const accountObject = account.reference("owner", ACCOUNT);
    await asOwner.create(accountObject, { owner });
    for (const [name, permissions] of [
        ["reader", [reading.permission("read")]],
        ["alice", [reading.permission("read"), reading.permission("unmask")]],
    ] as const) {
        const role = await asOwner.createRole(accountObject, {
            name,
            description: `What ${name} reads`,
            permissions: [...permissions],
        });
        await asOwner.grant({
            object: accountObject,
            role: role.id,
            subject: principal.user.reference("universe", name),
        });
    }

    // serve the readings, recording each unmasked read
    const unmasked: UnmaskedRead[] = [];
    const server = Server.start({
        ...implementEvents({
            store,
            access: { authorizer, database },
            unmasked: async (_context, read) => {
                unmasked.push(read);
            },
            ...(admit === undefined ? {} : { admit }),
        }),
        audience: meters.id,
        scope: "universe",
        resources: new ResourceContext(),
        health: new Health("event"),
        authenticate: async (request) => {
            const name = (request.headers.get("authorization") ?? "").slice("Bearer ".length);
            const subject = principal.user.reference("universe", name);

            return new Authentication({
                credential: { kind: "fixture", id: name },
                audience: meters.id,
                subject,
                subjects: [subject],
                verifiedAt: Date.now(),
                expiresAt: Date.now() + 60_000,
            });
        },
        authorizeMachine: async () => {},
        drainTimeout: 1000,
    });
    onTestFinished(() => server.close());
    const client = (name: string) =>
        createClient(eventService, {
            url: "http://events.local",
            headers: { authorization: `Bearer ${name}` },
            fetch: (request) => server.fetch(request),
        });

    return { store, client, unmasked };
}

/** A reading of the account taken by dana with a note. */
function taken(id: string, time: number) {
    return {
        scope: ACCOUNT,
        id,
        time,
        keys: { meter: "db.bytes", person: "dana" },
        data: { note: `checked ${id}` },
    };
}

test("read a kind as its access allows: masked for a reader, unmasked and recorded for one who may unmask, refused to a stranger, paged and exported newest first", async () => {
    // take two readings
    const { store, client, unmasked } = await serve();
    await store.append(meterReading, [taken("a", 1_000), taken("b", 2_000)]);

    // read them as the reader, alice and a stranger
    const selection = { kind: "reading", scope: ACCOUNT };
    const notes = (page: { readonly events: readonly { readonly data: unknown }[] }) =>
        page.events.map((event) => meterReading.parseData(event.data).note);
    const first = await client("reader").query({ ...selection, limit: 1 });
    const second = await client("reader").query({
        ...selection,
        limit: 1,
        ...(first.cursor === undefined ? {} : { cursor: first.cursor }),
    });
    const revealed = await client("alice").query(selection);
    const exported = [];
    for await (const event of await client("reader").export(selection)) {
        exported.push(meterReading.parseData(event.data).note);
    }
    const refused = await client("stranger")
        .query(selection)
        .then(
            () => "read",
            (error: unknown) => (isServiceError(error) ? error.code : String(error)),
        );

    expect({
        pages: [notes(first), notes(second)],
        revealed: notes(revealed),
        exported,
        refused,
        unmasked: unmasked.map((read) => [read.kind.name, read.operation, read.object.id]),
    }).toEqual({
        pages: [["****"], ["****"]],
        revealed: ["checked b", "checked a"],
        exported: ["****", "****"],
        refused: "FORBIDDEN",
        unmasked: [["reading", "query", ACCOUNT]],
    });
});

test("receive routed copies only from a host the admission takes, keeping each once", async () => {
    // serve one host admitting nobody and one admitting every reading
    const refusing = await serve();
    const admitted: string[] = [];
    const taking = await serve(async (_context, kind) => {
        admitted.push(kind.name);
    });

    // route one reading to each, the taking host twice
    const copy = { ...taken("a", 1_000), source: "account-01995da9-7223-7000-8000-000000000009" };
    const delivery = { kind: "reading", events: [copy] };
    const refused = await refusing
        .client("reader")
        .receive(delivery)
        .then(
            () => "received",
            (error: unknown) => (isServiceError(error) ? error.code : String(error)),
        );
    await taking.client("reader").receive(delivery);
    await taking.client("reader").receive(delivery);

    expect({
        refused,
        admitted,
        kept: (await taking.store.query(meterReading, { scope: ACCOUNT })).events.map((event) => [
            event.id,
            event.source,
        ]),
    }).toEqual({
        refused: "FORBIDDEN",
        admitted: ["reading", "reading"],
        kept: [["a", "account-01995da9-7223-7000-8000-000000000009"]],
    });
});

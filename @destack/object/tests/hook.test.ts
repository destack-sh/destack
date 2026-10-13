import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { defineDatabase, TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ResourceDeclaration } from "@destack/resource";
import { schema, Clock, present, type Identifier, type JsonValue } from "@destack/schema";
import type { Package } from "@destack/package";
import { definePackage, PACKAGE_DECLARATION, type PackageHandle } from "@destack/runtime";
import { describePackage } from "@destack/runtime/inspect";
import { RequestId } from "@destack/runtime/request";
import { testCallKey, testInstallation } from "@destack/runtime/test";
import { callTables, testEvents } from "../src/test/index.ts";
import {
    defineObject,
    field,
    type FileBucket,
    HandlerAnswer,
    HandlerRequest,
    HookServer,
    CallCode,
    type ObjectCall,
    ObjectType,
    SandboxHooks,
    SessionRequest,
} from "../src/index.ts";
import { ObjectExtension, type ObjectStore } from "../src/server/index.ts";
import type { InstalledDeclaration } from "../src/inspect/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { spaceId } from "./fixture/device.ts";
import { userContext } from "./fixture/user.ts";

/** Cards their owner remarks on. */
const card = defineObject({
    name: "card",
    plural: "cards",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        name: field.string(schema.string().min(1)),
    },
    permissions: { read: relation("owner"), write: relation("owner") },
}).methods((method) => ({
    create: method.create("write"),
    update: method.update("write"),
    remark: method.mutation({
        permission: "write",
        input: schema.object({ text: schema.string() }),
    }),
    stamp: method.update("write", { fields: ["name"], isInternal: true }),
}));

/** Notes of the cards' package, which no hook implements. */
const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), text: field.string() },
    permissions: { read: relation("owner"), write: relation("owner") },
}).methods((method) => ({ create: method.create("write") }));

/** The bucket the platform's photos keep their images in. */
const images: FileBucket = new ResourceDeclaration(card.package, {
    name: "images",
    kind: "bucket",
    spec: {},
});

/** Photos of the platform, each image kept as a file. */
const photo = defineObject({
    name: "photo",
    plural: "photos",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), image: field.file(images) },
    permissions: { read: relation("owner"), write: relation("owner") },
}).methods();

/** The package of the installed cards, apart from the platform's. */
const OTHER_PACKAGE = schema
    .identifier("package")
    .parse("package-01996ab0-0000-7000-8000-0000000000e1");

/** One message of a sandbox's exchange with its store, its session told apart by its order. */
type Message = readonly JsonValue[];

/** Send a value through JSON, as a request between processes carries it. */
function travel(value: unknown): unknown {
    return JSON.parse(JSON.stringify(value));
}

/** Serve cards whose hooks run in a sandbox of the package's installation. */
async function serveCards(cards: PackageHandle, owner: Package = card.package) {
    // keep the space's cards in memory
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...card.tables, ...note.tables, ...space.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await storage.database
        .insert(space.table)
        .values({ id: spaceId, scope: "universe", createdAt: 1, updatedAt: 1 });
    await openSpace(storage.database, spaceId);

    // name each session by its order, and record the exchange through JSON
    const exchange: Message[] = [];
    const sessions: string[] = [];
    const session = (token: string) => {
        if (!sessions.includes(token)) {
            sessions.push(token);
        }

        return `session ${sessions.indexOf(token) + 1}`;
    };

    // run the hooks in a sandbox reached through the store's sandbox hooks
    const installation = await testInstallation(owner, spaceId);
    const hooks: { sandbox?: SandboxHooks } = {};
    const server = new HookServer({
        packages: [cards],
        installation,
        session: async (request) => {
            const { mutation, request: asked } = SessionRequest.parse(travel(request));
            exchange.push([session(mutation), asked.kind, schema.json().parse(asked)]);
            const answer = await present(hooks.sandbox, "the sandbox").answer({
                mutation,
                request: asked,
            });
            exchange.push([session(mutation), "answer", summary(answer)]);

            return answer;
        },
    });
    const sandbox = new SandboxHooks({
        installation,
        declarations: declared(cards),
        send: async (request) => {
            const sent = HandlerRequest.parse(travel(request));
            const { call } = sent;
            exchange.push([
                session(sent.mutation),
                sent.hook.phase,
                { method: call.method, input: call.input, target: call.target?.["name"] ?? null },
            ]);
            const answer = HandlerAnswer.parse(travel(await server.answer(sent)));
            exchange.push([session(sent.mutation), "answer", summary(answer)]);

            return answer;
        },
    });
    hooks.sandbox = sandbox;

    // serve the cards as the build describes them, their routed hooks run in the sandbox
    const { store } = ObjectExtension.compose([], {
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: card.package },
        policies: [space],
        sandboxes: [sandbox],
    });

    return { store, exchange };
}

/** Describe the card type and the package routing it, as the cards' build declares them. */
function declared(cards: PackageHandle): InstalledDeclaration[] {
    return [
        ...Object.values(cards.objects)
            .filter((routed) => routed instanceof ObjectType)
            .map((type) => ({
                kind: "object",
                packageId: type.package.id,
                description: type.describe(),
            })),
        {
            kind: PACKAGE_DECLARATION,
            packageId: card.package.id,
            description: describePackage(cards),
        },
    ];
}

/** Move the card type of a declaration into another package, its scope kept in the platform's. */
function moved(
    entry: InstalledDeclaration,
    packageId: Identifier<"package">,
): InstalledDeclaration {
    // rewrite the declaration's own package and every reference to the card type
    const reference = schema.looseObject({
        packageId: schema.literal(card.package.id),
        type: schema.literal("card"),
    });
    const owner = schema.looseObject({ id: schema.literal(card.package.id) });
    const description: unknown = JSON.parse(
        JSON.stringify(entry.description),
        (_key, value: unknown): unknown => {
            const referenced = reference.safeParse(value);
            const owned = owner.safeParse(value);

            return referenced.success
                ? { ...referenced.data, packageId }
                : owned.success
                  ? { ...owned.data, id: packageId }
                  : value;
        },
    );

    return { ...entry, packageId, description: schema.json().parse(description) };
}

/** Summarize an answer: a result's card name, else the failure's code. */
function summary(answer: HandlerAnswer): JsonValue {
    if ("error" in answer) {
        return { error: answer.error.code };
    }
    const named = schema.looseObject({ name: schema.string() }).safeParse(answer.result);

    return named.success ? { name: named.data.name } : answer.result;
}

/** Run a call of a card method as a user, alice by default, answering the card. */
async function run(store: ObjectStore, call: ObjectCall, user = "alice") {
    const [result] = await store.mutate(userContext(user, spaceId).caller(), {
        id: RequestId.create(Date.now()),
        calls: [call],
    });

    return schema
        .looseObject({ id: schema.identifier("card"), name: schema.string() })
        .parse(result);
}

test("run a card's hooks in its sandbox, their reads, nested calls and wrapped behaviour answered in the mutation's session", async () => {
    // create a card whose sandboxed create hook capitalizes its name, then remark on it
    const { store, exchange } = await serveCards(
        definePackage({
            objects: {
                card: card.implement({
                    methods: {
                        create: (call, next) =>
                            next(
                                call.with({
                                    input: { ...call.input, name: call.input.name.toUpperCase() },
                                }),
                            ),
                        remark: async (call) => {
                            // read the card and rename it with the remark as the caller
                            const read = await call.read(card).row({ id: call.target.id });
                            await call.mutate(call.caller, [
                                card.call("update", {
                                    spaceId,
                                    id: call.target.id,
                                    name: `${present(read, "the card").name}: ${call.input.text}`,
                                }),
                            ]);

                            return call.target;
                        },
                    },
                }),
            },
        }),
    );
    const created = await run(store, card.call("create", { spaceId, name: "plan" }));
    const remarked = await run(
        store,
        card.call("remark", { spaceId, id: created.id, text: "ship it" }),
    );
    const stored = await store.query(
        userContext("alice", spaceId).caller(),
        card.call("get", { spaceId, id: created.id }),
    );

    // send each hook with its call, answer its next, read and nested call in its own session, and answer the hook
    expect({
        created: created.name,
        remarked: remarked.name,
        stored: stored.name,
        exchange,
    }).toEqual({
        created: "PLAN",
        remarked: "PLAN",
        stored: "PLAN: ship it",
        exchange: [
            ["session 1", "handle", { method: "create", input: { name: "plan" }, target: null }],
            ["session 1", "next", { kind: "next", input: { name: "PLAN" } }],
            ["session 1", "answer", { name: "PLAN" }],
            ["session 1", "answer", { name: "PLAN" }],
            [
                "session 2",
                "handle",
                { method: "remark", input: { text: "ship it" }, target: "PLAN" },
            ],
            [
                "session 2",
                "row",
                { kind: "row", table: card.table[TABLE].sqlName, key: { id: created.id } },
            ],
            ["session 2", "answer", { name: "PLAN" }],
            [
                "session 2",
                "call",
                {
                    kind: "call",
                    as: "caller",
                    call: {
                        type: card.typeReference,
                        method: "update",
                        input: { spaceId, id: created.id, name: "PLAN: ship it" },
                        release: card.package.version,
                    },
                    mutates: true,
                },
            ],
            ["session 2", "answer", { name: "PLAN: ship it" }],
            ["session 2", "answer", { name: "PLAN" }],
        ],
    });
});

test("run a sandboxed hook again in a fresh session once a concurrent commit changed what it read", async () => {
    // rename the card concurrently while the first attempt's hook runs
    const attempts: string[] = [];
    const served: { store?: ObjectStore } = {};
    const { store, exchange } = await serveCards(
        definePackage({
            objects: {
                card: card.implement({
                    methods: {
                        remark: async (call) => {
                            // read the card, renaming it concurrently on the first attempt
                            const read = await call.read(card).row({ id: call.target.id });
                            attempts.push(present(read, "the card").name);
                            if (attempts.length === 1) {
                                await run(
                                    present(served.store, "the store"),
                                    card.call("update", {
                                        spaceId,
                                        id: call.target.id,
                                        name: "Roadmap",
                                    }),
                                );
                            }

                            // rename it with the remark
                            await call.mutate(call.caller, [
                                card.call("update", {
                                    spaceId,
                                    id: call.target.id,
                                    name: `${present(read, "the card").name}: ${call.input.text}`,
                                }),
                            ]);

                            return call.target;
                        },
                    },
                }),
            },
        }),
    );
    served.store = store;
    const created = await run(store, card.call("create", { spaceId, name: "Plan" }));
    await run(store, card.call("remark", { spaceId, id: created.id, text: "ship it" }));
    const stored = await store.query(
        userContext("alice", spaceId).caller(),
        card.call("get", { spaceId, id: created.id }),
    );

    // read the concurrent name in the second attempt's own session, keeping only its change
    expect({
        attempts,
        stored: stored.name,
        sessions: exchange.filter(([, kind]) => kind === "handle").map(([name]) => name),
    }).toEqual({
        attempts: ["Plan", "Roadmap"],
        stored: "Roadmap: ship it",
        sessions: ["session 1", "session 2"],
    });
});

test("hide the cards a hook's caller may not read from the hook's reads", async () => {
    // remark by naming the card after the cards the hook lists and the other card it reads
    const other: { id?: string } = {};
    const { store } = await serveCards(
        definePackage({
            objects: {
                card: card.implement({
                    methods: {
                        remark: async (call) => {
                            const listed = await call.read(card).rows();
                            const read = await call
                                .read(card)
                                .row({ id: card.identifier(present(other.id, "bob's card")) });
                            const seen = { listed: listed.map((row) => row.name), read };
                            await call.mutate(call.caller, [
                                card.call("update", {
                                    spaceId,
                                    id: call.target.id,
                                    name: JSON.stringify(seen),
                                }),
                            ]);

                            return call.target;
                        },
                    },
                }),
            },
        }),
    );

    // create bob's card and alice's, then remark on alice's as alice
    const hidden = await run(store, card.call("create", { spaceId, name: "Secret" }), "bob");
    other.id = hidden.id;
    const created = await run(store, card.call("create", { spaceId, name: "Plan" }));
    await run(store, card.call("remark", { spaceId, id: created.id, text: "ship it" }));
    const stored = await store.query(
        userContext("alice", spaceId).caller(),
        card.call("get", { spaceId, id: created.id }),
    );

    // list alice's card alone, and read no card of bob's
    expect(JSON.parse(stored.name)).toEqual({ listed: ["Plan"], read: null });
});

/** Cards whose remark stamps the card through its internal method. */
const stamping = definePackage({
    objects: {
        card: card.implement({
            methods: {
                remark: async (call) => {
                    await call.mutate(call.caller, [
                        card.call("stamp", { spaceId, id: call.target.id, name: call.input.text }),
                    ]);

                    return call.target;
                },
            },
        }),
    },
});

test("call an internal method of the hook's own package in its session", async () => {
    // remark by stamping the card through its internal method
    const { store } = await serveCards(stamping);
    const created = await run(store, card.call("create", { spaceId, name: "Plan" }));
    const remarked = await run(
        store,
        card.call("remark", { spaceId, id: created.id, text: "ship it" }),
    );
    const stored = await store.query(
        userContext("alice", spaceId).caller(),
        card.call("get", { spaceId, id: created.id }),
    );

    expect({ remarked: remarked.name, stored: stored.name }).toEqual({
        remarked: "Plan",
        stored: "ship it",
    });
});

test("reach internal methods from the store's own code and its package's sandboxes alone", () => {
    // ask each code for the internal stamp and the public get
    const codes: readonly CallCode[] = [
        CallCode.store,
        CallCode.client,
        { kind: "sandbox", packageId: card.package.id },
        { kind: "sandbox", packageId: OTHER_PACKAGE },
    ];
    const reached = codes.map((code) =>
        ["stamp", "get"].map((name) => CallCode.reaches(code, card, card.method(name))),
    );

    expect(reached).toEqual([
        [true, true],
        [false, true],
        [true, true],
        [false, true],
    ]);
});

test("read the rows of a type the installation serves without hooks", async () => {
    // remark by naming the card after the caller's notes
    const { store } = await serveCards(
        definePackage({
            objects: {
                card: card.implement({
                    methods: {
                        remark: async (call) => {
                            const notes = await call.read(note).rows();
                            await call.mutate(call.caller, [
                                card.call("update", {
                                    spaceId,
                                    id: call.target.id,
                                    name: notes.map((row) => row.text).join(", "),
                                }),
                            ]);

                            return call.target;
                        },
                    },
                }),
                note,
            },
        }),
    );

    // write a note and a card as alice, then remark on the card
    const caller = userContext("alice", spaceId).caller();
    await store.mutate(caller, {
        id: RequestId.create(Date.now()),
        calls: [note.call("create", { spaceId, text: "ship it" })],
    });
    const created = await run(store, card.call("create", { spaceId, name: "Plan" }));
    await run(store, card.call("remark", { spaceId, id: created.id, text: "now" }));
    const stored = await store.query(caller, card.call("get", { spaceId, id: created.id }));

    expect(stored.name).toBe("ship it");
});

test("refuse serving the cards of two installations of one package from one store", async () => {
    // install the cards twice in the space, each in a sandbox of its own
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...card.tables, ...note.tables, ...space.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const cards = definePackage({ objects: { card } });
    const installed = await testInstallation(card.package, spaceId);
    const sandbox = (id: string) =>
        new SandboxHooks({
            installation: { ...installed, id },
            declarations: declared(cards),
            send: () => Promise.reject(new TypeError("the cards have no hooks")),
        });
    const [first, second] = [
        "installation-01996ab0-0000-7000-8000-0000000000d1",
        "installation-01996ab0-0000-7000-8000-0000000000d2",
    ];

    // compose a store serving both
    const events = await testEvents(storage.database);
    const compose = () =>
        ObjectExtension.compose([], {
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            database: storage.database,
            callKey: testCallKey,
            events,
            origin: { package: card.package },
            policies: [space],
            sandboxes: [sandbox(first), sandbox(second)],
        });

    expect(compose).toThrow(
        new TypeError(
            `installation ${first} and installation ${second} both implement card objects of ${card.package.id}`,
        ),
    );
});

test("route the platform's cards from an installed package to their one implementation, and refuse a route nothing implements", async () => {
    // let an installed board package route the platform's cards, which it does not implement
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...card.tables, ...note.tables, ...space.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const cards = definePackage({ objects: { card } });
    const board = { ...card.package, id: OTHER_PACKAGE, name: "@app/board" };
    const boards = definePackage({ objects: { card } }, { package: board });
    const installation = await testInstallation(board, spaceId);
    const sandbox = new SandboxHooks({
        installation,
        declarations: [
            {
                kind: PACKAGE_DECLARATION,
                packageId: board.id,
                description: describePackage(boards),
            },
        ],
        send: () => Promise.reject(new TypeError("the board has no hooks")),
    });
    const events = await testEvents(storage.database);
    const compose = (extensions: readonly ObjectExtension[]) =>
        ObjectExtension.compose(extensions, {
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            database: storage.database,
            callKey: testCallKey,
            events,
            origin: { package: card.package },
            policies: [space],
            sandboxes: [sandbox],
        });

    // serve the cards once, as the platform's, under both packages
    const { store, packages } = compose([ObjectExtension.of(cards, [])]);
    expect({
        routed: store.routed.map((type) => type.typeReference),
        installed: store.installationOf(card)?.id,
        routes: packages.map((served) => Object.keys(served.router)),
    }).toEqual({
        routed: [{ packageId: card.package.id, type: "card" }],
        installed: undefined,
        routes: [
            ["replica", "card"],
            ["replica", "card"],
        ],
    });

    // refuse the board's route of cards without the platform's implementation
    expect(() => compose([])).toThrow(
        new TypeError(
            `installation ${installation.id} routes card objects of ${card.package.id}, which nothing here implements or copies`,
        ),
    );
});

test("serve the cards of the platform and of an installed package from one store, each under its own package", async () => {
    // declare an installed package's own card type beside the platform's
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...card.tables, ...note.tables, ...space.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const cards = definePackage({ objects: { card } });
    const installed = { ...card.package, id: OTHER_PACKAGE, name: "@app/cards" };
    const sandbox = new SandboxHooks({
        installation: await testInstallation(installed, spaceId),
        declarations: declared(cards).map((entry) => moved(entry, installed.id)),
        send: () => Promise.reject(new TypeError("the cards have no hooks")),
    });

    // compose a store serving the platform's cards and the installation's
    const { store, packages } = ObjectExtension.compose([ObjectExtension.of(cards, [])], {
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: card.package },
        policies: [space],
        sandboxes: [sandbox],
    });

    // serve both types, each package routing its own under the card route
    expect(store.routed.map((type) => type.typeReference)).toEqual([
        { packageId: card.package.id, type: "card" },
        { packageId: OTHER_PACKAGE, type: "card" },
    ]);
    expect(packages.map((served) => Object.keys(served.router))).toEqual([
        ["replica", "card"],
        ["replica", "card"],
    ]);
});

test("describe the platform's and an installed package's types from one store, declaring each from the descriptions alone", async () => {
    // serve the platform's photos and an installed package's cards from one store
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...photo.tables, ...card.tables, ...space.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const photos = definePackage({ objects: { photo } });
    const cards = definePackage({ objects: { card } });
    const installed = { ...card.package, id: OTHER_PACKAGE, name: "@app/cards" };
    const sandbox = new SandboxHooks({
        installation: await testInstallation(installed, spaceId),
        declarations: declared(cards).map((entry) => moved(entry, installed.id)),
        send: () => Promise.reject(new TypeError("the cards have no hooks")),
    });
    const { store } = ObjectExtension.compose([ObjectExtension.of(photos, [])], {
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: card.package },
        policies: [space],
        sandboxes: [sandbox],
    });

    // declare every type the store describes, the photo's image kept in the platform's bucket
    const types = ObjectType.ofAll(store.describe(), []);
    const described = present(
        types.find((type) => type.name === "photo"),
        "the described photo type",
    );
    expect({
        types: types.map((type) => [type.package.id, type.name]),
        bucket: described.fields["image"]?.bucket,
    }).toEqual({
        types: [
            [OTHER_PACKAGE, "card"],
            [space.package.id, "space"],
            [card.package.id, "photo"],
        ],
        bucket: { packageId: card.package.id, name: "images" },
    });
});

test("list the phases a package attaches as hooks, leaving a trait's methods to every store", () => {
    // declare controlled machines handling their creation, and implement the trait's observe
    const machine = defineObject({
        name: "machine",
        plural: "machines",
        scope: space,
        controlled: true,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner"), write: relation("owner") },
    }).methods((method) => ({
        create: method.create("write").handle((call, next) => next(call)),
    }));
    const implemented = machine.implement({ methods: { observe: { authorize: async () => {} } } });

    // list the package's own phases alone
    expect({ declared: machine.hooks, implemented: implemented.hooks }).toEqual({
        declared: { create: ["handle"] },
        implemented: { create: ["handle"], observe: ["authorize"] },
    });
});

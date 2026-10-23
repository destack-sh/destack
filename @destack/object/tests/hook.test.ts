import { Failure } from "@destack/error";
import {
    CallSite,
    defineObject,
    field,
    type FileBucket,
    HandlerAnswer,
    HandlerRequest,
    HookSandbox,
    HookServer,
    ObjectDescription,
    ObjectType,
    ObjectTypePool,
    SessionRequest,
} from "../src/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import type { Call } from "@destack/sync";
import { principal, relation } from "@destack/access";
import { defineDatabase, TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ResourceDeclaration } from "@destack/resource";
import { schema, present, type JsonValue } from "@destack/schema";
import { graph, Package } from "@destack/package";
import { definePackage, PACKAGE_DECLARATION, PackageHandle } from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import { RequestId } from "@destack/sync";
import { testCallKey, testServerContext } from "@destack/object/test";
import { testEvents } from "../src/test/index.ts";
import { implementPackage, ObjectStore } from "../src/server/index.ts";

/** A declaration of a build, as a sandbox receives it. */
type Declared = Pick<graph.Declaration, "symbol" | "kind" | "description">;
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { spaceId } from "./fixture/device.ts";
import { userContext, users } from "./fixture/user.ts";
import { Clock } from "@destack/environment";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** Cards their owner remarks on. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(schema.string().min(1)),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write"),
        remark: method.mutation({
            permission: "write",
            input: schema.object({ text: schema.string() }),
        }),
        stamp: method.update("write", { fields: ["name"], isInternal: true }),
    }),
);

/** Notes of the cards' package, which no hook implements. */
const note = defineObject(
    {
        name: "note",
        plural: "notes",
        scope: space,
        fields: { owner: field.reference(principal.user).caller(), text: field.string() },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({ create: method.create("write") }),
);

/** The bucket the platform's photos store their images in. */
const images: FileBucket = new ResourceDeclaration(card.package, {
    name: "images",
    kind: "bucket",
    spec: {},
});

/** Photos of the platform, each image stored as a file. */
const photo = defineObject({
    name: "photo",
    plural: "photos",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), image: field.file(images) },
    permissions: { read: relation("owner"), write: relation("owner") },
});

/** The package of the installed cards, apart from the platform's. */
const OTHER_PACKAGE = schema
    .identifier("package")
    .parse("package-01996ab0-0000-7000-8000-0000000000e1");

/** One message of a sandbox's exchange with its store, its session told apart by its order. */
type Message = readonly JsonValue[];

/** Send a value through JSON, as a request between processes sends it. */
function travel(value: unknown): unknown {
    return JSON.parse(JSON.stringify(value));
}

/** Serve cards whose hooks run in a sandbox of the package's installation. */
async function serveCards(cards: PackageHandle, owner: Package = card.package) {
    // store the space's cards in memory
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { card, note, space }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await storage.database
        .insert(space.table)
        .values({ id: spaceId, scope: "universe", createdAt: 1, updatedAt: 1 });
    await openSpace(storage.database, spaceId);

    // label each session by its order, and record the exchange through JSON
    const exchange: Message[] = [];
    const sessions: string[] = [];
    const session = (token: string) => {
        if (!sessions.includes(token)) {
            sessions.push(token);
        }

        return `session ${sessions.indexOf(token) + 1}`;
    };

    // run the hooks in a sandbox called through the store's sandbox hooks
    const context = await testServerContext(owner, spaceId);
    const hooks: { sandbox?: HookSandbox } = {};
    const server = new HookServer({
        packages: [cards],
        server: context,
        session: async (request) => {
            const { mutation, request: asked } = SessionRequest.parse(travel(request));
            exchange.push([session(mutation), asked.kind, schema.json().parse(asked)]);
            const answer = await present(hooks.sandbox, "the sandbox").answer({
                mutation,
                request: asked,
            });
            exchange.push([session(mutation), "answer", describe(answer)]);

            return answer;
        },
    });
    const sandbox = new HookSandbox({
        server: context,
        declarations: [...declared(cards), dependency],
        send: async (request) => {
            const sent = HandlerRequest.parse(travel(request));
            const { call } = sent;
            exchange.push([
                session(sent.mutation),
                sent.hook.phase,
                { method: call.method, input: call.input, target: call.target?.["name"] ?? null },
            ]);
            const answer = HandlerAnswer.parse(travel(await server.answer(sent)));
            exchange.push([session(sent.mutation), "answer", describe(answer)]);

            return answer;
        },
    });
    hooks.sandbox = sandbox;

    // serve the cards as the build describes them, their routed hooks run in the sandbox
    const store = new ObjectStore({
        servers: [],
        chain: async () => undefined,
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
function declared(cards: PackageHandle): Declared[] {
    const symbol = graph.Moniker.of({ packageId: cards.id, module: "src/index.ts", name: "cards" });

    return [
        ...Object.values(cards.objects)
            .filter((routed): routed is ObjectType => routed instanceof ObjectType)
            .map((type) => ({ symbol, kind: "object", description: ObjectDescription.of(type) })),
        { symbol, kind: PACKAGE_DECLARATION, description: describePackage(cards) },
    ];
}

/** A bundled dependency of the cards' package. */
const bundled = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d9",
    name: "@example/dependency",
    version: "2026.9.0",
});

/** A sticker the bundled dependency declares. */
const sticker = defineObject(
    {
        name: "sticker",
        plural: "stickers",
        scope: space,
        fields: {
            /** The sticker's text. */
            text: field.string(),
        },
        permissions: ["read"],
    },
    () => ({}),
    { package: bundled },
);

/** The bundled dependency's package declaration, at its symbol, which no sandbox serves. */
const dependency: Declared = {
    symbol: graph.Moniker.of({ packageId: bundled.id, module: "src/index.ts", name: "default" }),
    kind: PACKAGE_DECLARATION,
    description: describePackage(PackageHandle.of(bundled, { objects: { sticker } })),
};

/** Move the card type of a declaration into another package, its scope left in the platform's. */
function moved(entry: Declared, installed: Package): Declared {
    // rewrite the declaration's package and every reference to the card type
    const reference = schema.looseObject({
        packageId: schema.literal(card.package.id),
        type: schema.literal("card"),
    });
    const owner = schema.looseObject({ id: schema.literal(card.package.id) });
    const description: unknown = JSON.parse(
        JSON.stringify(entry.description),
        (_key, value: unknown): unknown => {
            const referenced = reference.safeParse(value);
            const parsed = owner.safeParse(value);

            return referenced.success
                ? { ...referenced.data, packageId: installed.id }
                : parsed.success
                  ? { ...parsed.data, id: installed.id, name: installed.name }
                  : value;
        },
    );

    const symbol = graph.Moniker.of({
        packageId: installed.id,
        module: "src/index.ts",
        name: "cards",
    });

    return { ...entry, symbol, description: schema.jsonObject().parse(description) };
}

/** Summarize an answer: a result's card name, else the failure's code. */
function describe(answer: HandlerAnswer): JsonValue {
    if ("error" in answer) {
        return { error: answer.error.code };
    }
    const named = schema.looseObject({ name: schema.string() }).safeParse(answer.result);

    return named.success ? { name: named.data.name } : answer.result;
}

/** Run a call of a card method as a user, alice by default, answering the card. */
async function run(store: ObjectStore, call: Call, user = users.alice) {
    const [result] = await store.mutate(userContext(user, spaceId).access(), {
        id: RequestId.create(Date.now()),
        calls: [call],
    });

    return schema
        .looseObject({ id: schema.identifier("card"), name: schema.string() })
        .parse(result);
}

test("run a card's hooks in its sandbox, their reads, nested calls and wrapped behaviour responded with in the mutation's session", async () => {
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
                            const read = await call
                                .read(card)
                                .findFirst({ where: { id: call.target.id } });
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
        userContext(users.alice, spaceId).access(),
        card.call("get", { spaceId, id: created.id }),
    );

    // send each hook with its call, answer its next, read and nested call in a new session, then the hook
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
                "findFirst",
                {
                    kind: "findFirst",
                    table: card.table[TABLE].sqlName,
                    where: { id: created.id },
                    columns: "logged",
                },
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
                            const read = await call
                                .read(card)
                                .findFirst({ where: { id: call.target.id } });
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
        userContext(users.alice, spaceId).access(),
        card.call("get", { spaceId, id: created.id }),
    );

    // read the concurrent name in the second attempt's session, seeing only its change
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

test("read every card of the package in a hook, and read as the caller through a query", async () => {
    // title a remark after the cards the hook lists and reads, and the caller's query
    const other: { id?: string } = {};
    const { store } = await serveCards(
        definePackage({
            objects: {
                card: card.implement({
                    methods: {
                        remark: async (call) => {
                            const listed = await call.read(card).findMany();
                            const read = await call.read(card).findFirst({
                                where: { id: card.identifier(present(other.id, "bob's card")) },
                            });
                            const asCaller = await call
                                .query(
                                    call.caller,
                                    card.call("get", {
                                        spaceId,
                                        id: present(other.id, "bob's card"),
                                    }),
                                )
                                .then(
                                    () => "found",
                                    (error: unknown) =>
                                        schema.looseObject({ code: schema.string() }).parse(error)
                                            .code,
                                );
                            const seen = {
                                listed: listed.map((row) => row.name).toSorted(),
                                read: read?.name ?? null,
                                asCaller,
                            };
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
    const hidden = await run(store, card.call("create", { spaceId, name: "Secret" }), users.bob);
    other.id = hidden.id;
    const created = await run(store, card.call("create", { spaceId, name: "Plan" }));
    await run(store, card.call("remark", { spaceId, id: created.id, text: "ship it" }));
    const stored = await store.query(
        userContext(users.alice, spaceId).access(),
        card.call("get", { spaceId, id: created.id }),
    );

    // list and read bob's card as the package, and find it hidden from alice's query
    expect(JSON.parse(stored.name)).toEqual({
        listed: ["Plan", "Secret"],
        read: "Secret",
        asCaller: "NOT_FOUND",
    });
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

test("call an internal method of the hook's package in its session", async () => {
    // remark by stamping the card through its internal method
    const { store } = await serveCards(stamping);
    const created = await run(store, card.call("create", { spaceId, name: "Plan" }));
    const remarked = await run(
        store,
        card.call("remark", { spaceId, id: created.id, text: "ship it" }),
    );
    const stored = await store.query(
        userContext(users.alice, spaceId).access(),
        card.call("get", { spaceId, id: created.id }),
    );

    expect({ remarked: remarked.name, stored: stored.name }).toEqual({
        remarked: "Plan",
        stored: "ship it",
    });
});

test("decide permissions, take positions, read the start position and act as the installation through a hook's session", async () => {
    // remark by deciding the caller's permissions and drawing from the mutation as the sandbox sees it
    const seen: JsonValue[] = [];
    const { store, exchange } = await serveCards(
        definePackage({
            objects: {
                card: card.implement({
                    methods: {
                        remark: async (call) => {
                            const reference = card.reference(call.scope, call.target.id);
                            const decision = await call.check(card.permission("write"), reference);
                            await call.require(card.permission("write"), reference);
                            const id = await call.generateId(note);
                            seen.push({
                                isAllowed: decision.isAllowed,
                                isNote: note.idSchema.safeParse(id).success,
                                hasBase: typeof call.base === "object",
                                self: schema.json().parse(call.self.subject ?? null),
                            });

                            return call.target;
                        },
                    },
                }),
            },
        }),
    );
    const created = await run(store, card.call("create", { spaceId, name: "Plan" }));
    await run(store, card.call("remark", { spaceId, id: created.id, text: "ship it" }));
    const context = await testServerContext(card.package, spaceId);

    // answer the check, the requirement and the position in the hook's session
    const reference = { packageId: card.package.id, type: "card", scope: spaceId, id: created.id };
    const write = { packageId: card.package.id, type: "card", name: "write" };
    expect({ seen, exchange: exchange.filter(([, kind]) => kind !== "handle") }).toEqual({
        seen: [
            {
                isAllowed: true,
                isNote: true,
                hasBase: true,
                self: schema
                    .json()
                    .parse(principal.installation.reference(spaceId, context.installation)),
            },
        ],
        exchange: [
            ["session 1", "check", { kind: "check", permission: write, object: reference }],
            ["session 1", "answer", { isAllowed: true }],
            ["session 1", "require", { kind: "require", permission: write, object: reference }],
            ["session 1", "answer", null],
            ["session 1", "position", { kind: "position" }],
            ["session 1", "answer", 0],
            ["session 1", "answer", { name: "Plan" }],
        ],
    });
});

test("serve the installation's package from its sandbox, never a bundled dependency's", async () => {
    const { store } = await serveCards(definePackage({ objects: { card } }));

    expect(store.packages.map((served) => served.package.id)).toEqual([card.package.id]);
});

test("call internal methods from the store's code and its package's sandboxes alone", () => {
    // ask each code for the internal stamp and the public get
    const codes: readonly CallSite[] = [
        CallSite.store,
        CallSite.client,
        { kind: "sandbox", packageId: card.package.id },
        { kind: "sandbox", packageId: OTHER_PACKAGE },
    ];
    const covered = codes.map((code) =>
        ["stamp", "get"].map((name) => CallSite.covers(code, card, card.method(name))),
    );

    expect(covered).toEqual([
        [true, true],
        [false, true],
        [true, true],
        [false, true],
    ]);
});

test("read the rows of a type the installation serves without hooks", async () => {
    // remark by titling the card after the caller's notes
    const { store } = await serveCards(
        definePackage({
            objects: {
                card: card.implement({
                    methods: {
                        remark: async (call) => {
                            const notes = await call.read(note).findMany();
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
    const caller = userContext(users.alice, spaceId).access();
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
    // install the cards twice in the space, each in a separate sandbox
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { card, note, space }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const cards = definePackage({ objects: { card } });
    const installed = await testServerContext(card.package, spaceId);
    const sandbox = (id: string) =>
        new HookSandbox({
            server: { ...installed, installation: id },
            declarations: declared(cards),
            send: () => Promise.reject(new TypeError("the cards have no hooks")),
        });
    const [first, second] = [
        "installation-01996ab0-0000-7000-8000-0000000000d1",
        "installation-01996ab0-0000-7000-8000-0000000000d2",
    ];

    // serve both from one store
    const events = await testEvents(storage.database);
    const serve = () =>
        new ObjectStore({
            servers: [],
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            database: storage.database,
            callKey: testCallKey,
            events,
            origin: { package: card.package },
            policies: [space],
            sandboxes: [sandbox(first), sandbox(second)],
        });

    expect(serve).toThrow(
        new TypeError(
            `installation ${first} and installation ${second} both implement card objects of ${card.package.id}`,
        ),
    );
});

test("serve the cards of the platform and of an installed package from one store, each under its package", async () => {
    // declare an installed package's card type beside the platform's
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { card, note, space }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const cards = definePackage({ objects: { card } });
    const installed = { ...card.package, id: OTHER_PACKAGE, name: "@app/cards" };
    const sandbox = new HookSandbox({
        server: await testServerContext(installed, spaceId),
        declarations: declared(cards).map((entry) => moved(entry, installed)),
        send: () => Promise.reject(new TypeError("the cards have no hooks")),
    });

    // serve the platform's cards and the installation's from one store
    const store = new ObjectStore({
        servers: [implementPackage(cards, {})],
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: card.package },
        policies: [space],
        sandboxes: [sandbox],
    });

    // serve both types, each package routing its type under the card route
    expect(store.objects.map((type) => type.typeReference)).toEqual([
        { packageId: card.package.id, type: "card" },
        { packageId: OTHER_PACKAGE, type: "card" },
    ]);
    expect(store.packages.map((served) => Object.keys(served.router))).toEqual([
        ["store", "card"],
        ["store", "card"],
    ]);
});

test("describe the platform's and an installed package's types from one store, declaring each from the descriptions alone", async () => {
    // serve the platform's photos and an installed package's cards from one store
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { photo, card, space }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const photos = definePackage({ objects: { photo } });
    const cards = definePackage({ objects: { card } });
    const installed = { ...card.package, id: OTHER_PACKAGE, name: "@app/cards" };
    const sandbox = new HookSandbox({
        server: await testServerContext(installed, spaceId),
        declarations: declared(cards).map((entry) => moved(entry, installed)),
        send: () => Promise.reject(new TypeError("the cards have no hooks")),
    });
    const store = new ObjectStore({
        servers: [implementPackage(photos, {})],
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: card.package },
        policies: [space],
        sandboxes: [sandbox],
    });

    // declare every type the store describes, the photo's image stored in the platform's bucket
    const types = ObjectTypePool.of([]).declare(store.describe());
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
    const machine = defineObject(
        {
            name: "machine",
            plural: "machines",
            scope: space,
            controlled: true,
            fields: { owner: field.reference(principal.user).caller() },
            permissions: { read: relation("owner"), write: relation("owner") },
        },
        (method) => ({
            create: method.create("write").handle((context, next) => next(context)),
        }),
    );
    const implemented = machine.implement({ methods: { observe: { authorize: async () => {} } } });

    // list only the package's phases
    expect({ declared: machine.hooks, implemented: implemented.hooks }).toEqual({
        declared: { create: ["handle"] },
        implemented: { create: ["handle"], observe: ["authorize"] },
    });
});

test("throw a hook's failure out of its sandbox as the failure it was, caused at the stack it was thrown at", async () => {
    // fail a hook, and read its answer as JSON on the other side
    const thrown = new TypeError("the card has no title");
    const answer = HandlerAnswer.parse(
        JSON.parse(
            JSON.stringify(
                await HandlerAnswer.of(() => {
                    throw thrown;
                }),
            ),
        ),
    );

    // throw its code, status and message, caused by an error at the thrown stack
    const failed = (() => {
        try {
            return HandlerAnswer.result(answer);
        } catch (error) {
            return error;
        }
    })();
    const cause = failed instanceof Error ? failed.cause : undefined;
    expect([
        Failure.of(failed),
        cause instanceof Error ? [cause.message, cause.stack] : cause,
    ]).toEqual([
        Failure.http("INTERNAL_SERVER_ERROR", "the card has no title"),
        ["the card has no title", thrown.stack],
    ]);
});

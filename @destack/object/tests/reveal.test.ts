import { testSimulation } from "../src/test/index.ts";
import { through } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { PackageVersion } from "@destack/package";
import { CallError } from "@destack/object";
import { RequestId } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { schema, Uuid } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { defineObject, field } from "../src/index.ts";
import { testEvents, serversOf } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The package of the types reading the holders from outside their package. */
const OUTSIDE = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-000000000071",
    name: "@example/outside",
    version: "2026.10.0",
});

/** A holder whose token its package's code reads, which another package's read withholds. */
const holder = defineObject(
    {
        name: "holder",
        plural: "holders",
        title: t`${plural(argument("count"), { one: "Holder", other: "Holders" })}`,
        scope: space,
        fields: { label: field.string(), token: field.string().sensitive() },
        access: {
            permissions: { read: through("space", "read") },
        },
    },
    (method) => ({
        create: method.create("read", { fields: ["label", "token"] }),
        /** Read a holder's token as the holders' package. */
        token: method
            .query({
                title: t`Token`,
                permission: "read",
                output: schema.object({ token: schema.string() }),
            })
            .handle(async (context) => {
                const [row] = await context
                    .reveal(context.object)
                    .findMany({ where: { id: context.target.id } });

                return { token: schema.string().parse(row?.["token"]) };
            }),
    }),
);

/** A peeker of another package, reading the holders of its scope. */
const peeker = defineObject(
    {
        name: "peeker",
        plural: "peekers",
        title: t`${plural(argument("count"), { one: "Peeker", other: "Peekers" })}`,
        scope: space,
        fields: { label: field.string() },
        access: {
            permissions: { read: through("space", "read") },
        },
    },
    (method) => ({
        /** Read the scope's holders as the outside package, then try to reveal them. */
        peek: method
            .query({
                title: t`Peek`,
                permission: "read",
                target: "scope",
                output: schema.object({
                    hasToken: schema.array(schema.boolean()),
                    revealed: schema.string(),
                }),
            })
            .handle(async (context) => {
                const scope = schema.identifier("space").parse(context.scope);
                const read = await context.read(holder).findMany({ where: { scope } });
                const revealed = await Promise.resolve()
                    .then(() => context.reveal(holder).findMany({ where: { scope } }))
                    .then(
                        () => "revealed",
                        (error: unknown) => (error instanceof Error ? error.message : "refused"),
                    );

                return { hasToken: read.map((row) => Object.hasOwn(row, "token")), revealed };
            }),
    }),
    { package: OUTSIDE },
);

test("reveal a type's sensitive field to its package's handler, and withhold it from another package's read", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve the holders and the outside peekers in one space
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { holder, peeker }, tables: [] }],
                follows: [],
                tables: [space.table],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const spaceId = `space-${Uuid.v7(world)}`;
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([holder, peeker]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: holder.package },
    });
    await openSpace(storage.database, world, spaceId);
    const caller = userContext(users.alice, spaceId, { environment: world }).access();

    // create a holder with a token
    const [created] = await store.mutate(caller, {
        id: RequestId.create(world),
        calls: [holder.call("create", { spaceId, label: "deploy", token: "token-0123" })],
    });
    const id = schema.looseObject({ id: schema.string() }).parse(created).id;

    // read its token as its package, and its rows from the outside package
    const inside = await store.query(caller, holder.call("token", { spaceId, id }));
    const outside = await store.query(caller, peeker.call("peek", { spaceId }));

    // answer the token to the holders' code, and leave it out of the outside read and reveal
    expect({ inside, outside }).toEqual({
        inside: { token: "token-0123" },
        outside: {
            hasToken: [false],
            revealed: CallError.http("FORBIDDEN", {
                message: `type holder is forbidden: '@example/outside' cannot reveal it, as only the declaring package can`,
            }).message,
        },
    });
});

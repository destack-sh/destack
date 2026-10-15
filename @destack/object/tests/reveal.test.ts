import { through } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Package } from "@destack/package";
import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { testCallKey } from "@destack/runtime/test";
import { Clock, schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";

/** The package of the types reading the holders from outside their package. */
const OUTSIDE = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000071",
    name: "@example/outside",
    version: "2026.10.0",
});

/** A holder whose token its own package's code reads, which another package's read withholds. */
const holder = defineObject({
    name: "holder",
    plural: "holders",
    scope: space,
    fields: { label: field.string(), token: field.string().sensitive() },
    permissions: { read: through("space", "read") },
}).methods((method) => ({
    create: method.create("read", { fields: ["label", "token"] }),
    /** Read a holder's token as the holders' own package. */
    token: method
        .query({
            permission: "read",
            output: schema.object({ token: schema.string() }),
        })
        .handle(async (call) => {
            const [row] = await call
                .reveal(call.object)
                .findMany({ where: { id: call.target.id } });

            return { token: schema.string().parse(row?.["token"]) };
        }),
}));

/** A peeker of another package, reading the holders of its scope. */
const peeker = defineObject(
    {
        name: "peeker",
        plural: "peekers",
        scope: space,
        fields: { label: field.string() },
        permissions: { read: through("space", "read") },
    },
    { package: OUTSIDE },
).methods((method) => ({
    /** Read the scope's holders as the outside package, then try to reveal them. */
    peek: method
        .query({
            permission: "read",
            target: "scope",
            output: schema.object({
                hasToken: schema.array(schema.boolean()),
                revealed: schema.string(),
            }),
        })
        .handle(async (call) => {
            const scope = schema.identifier("space").parse(call.scope);
            const read = await call.read(holder).findMany({ where: { scope } });
            const revealed = await Promise.resolve()
                .then(() => call.reveal(holder).findMany({ where: { scope } }))
                .then(
                    () => "revealed",
                    (error: unknown) => (error instanceof Error ? error.message : "refused"),
                );

            return { hasToken: read.map((row) => Object.hasOwn(row, "token")), revealed };
        }),
}));

test("reveal a type's sensitive field to its own package's handler, and withhold it from another package's read", async () => {
    // serve the holders and the outside peekers in one space
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...holder.tables, ...peeker.tables, space.table, ...callTables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const spaceId = `space-${v7()}`;
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([holder, peeker]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: holder.package },
    });
    await openSpace(storage.database, spaceId);
    const caller = userContext(users.alice, spaceId).access();

    // create a holder with a token
    const [created] = await store.mutate(caller, {
        id: RequestId.create(Date.now()),
        calls: [holder.call("create", { spaceId, label: "deploy", token: "token-0123" })],
    });
    const id = schema.looseObject({ id: schema.string() }).parse(created).id;

    // read its token as its own package, and its rows from the outside package
    const own = await store.query(caller, holder.call("token", { spaceId, id }));
    const outside = await store.query(caller, peeker.call("peek", { spaceId }));

    // answer the token to the holders' own code, and leave it out of the outside read and reveal
    expect({ own, outside }).toEqual({
        own: { token: "token-0123" },
        outside: {
            hasToken: [false],
            revealed: new CallError("FORBIDDEN", {
                message: `@example/outside reveals holder only in its own package`,
            }).message,
        },
    });
});

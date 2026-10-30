import { testSimulation } from "../src/test/index.ts";
import { principal, relation } from "@destack/access";
import { bucket } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { RequestId, Scope } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { type Identifier, schema } from "@destack/schema";
import { CallError } from "@destack/object";
import { expect, onTestFinished, test } from "@destack/test";
import { defineObject, field } from "../src/index.ts";
import { serversOf, testEvents } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space the posts are written in. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** Posts each author writes at most two of a minute. */
const post = defineObject(
    {
        name: "post",
        plural: "posts",
        scope: space,
        fields: { author: field.reference(principal.user), title: field.string() },
        permissions: { read: relation("author"), write: relation("author") },
        limits: { create: { rate: { count: 2, every: { minutes: 1 } }, key: "caller" } },
    },
    (method) => ({ create: method.create("write") }),
);

test("refuse a third post of an author within a minute, saying when the next is due, and allow others", async () => {
    // run in a world of its own
    const simulation = testSimulation(Date.UTC(2026, 9, 16, 9));
    const { clock } = simulation;
    const world = simulation.environment("test");

    // serve the posts on a simulated clock
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { post }, tables: [] }],
            follows: [],
            tables: [bucket],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([post]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: post.package },
    });
    const write = (author: Identifier<"user">) =>
        store.mutate(userContext(author, spaceId, { environment: world }).access(), {
            id: RequestId.create(world),
            calls: [post.call("create", { spaceId, authorId: author, title: "Hello" })],
        });

    // write two posts as alice, refuse her third, allow bob's, and allow alice again half a minute later
    await write(users.alice);
    await write(users.alice);
    const third = await write(users.alice).then(
        () => undefined,
        (error: unknown) => {
            const known = CallError.of(error);

            return known === undefined ? error : [known.code, known.message, known.data];
        },
    );
    await write(users.bob);
    await simulation.jump(clock.now() + 30_000);
    await write(users.alice);

    // expect the third due once the 30 second interval of two posts a minute passed
    expect(third).toEqual([
        "TOO_MANY_REQUESTS",
        "post.create is limited to 2 calls per 60000 ms",
        { retryAfter: 30_000 },
    ]);
});

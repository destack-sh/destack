import { Snapshot } from "@destack/db";
import { AccessContext, Caller } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { Package } from "@destack/package";
import { RequestId, Scope, type Publisher } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { present, schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { v7 } from "uuid";
import { defineShape, Follow } from "../src/index.ts";
import { implementPackage, ObjectStore } from "../src/server/index.ts";
import { testEvents } from "../src/test/index.ts";
import { page, pageDatabase, pagesPackage } from "./fixture/page.ts";
import { openSpace, spacePrincipal } from "./fixture/space.ts";
import { Clock } from "@destack/environment";

/** The space with the pages. */
const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** The pages of a title, wherever their scope has them. */
const titled = defineShape({
    name: "titled",
    parameters: schema.object({ title: schema.string() }),
    objects: [page],
    queries: ({ title }) => [page.query({ where: { title } })],
});

/** The pages with an icon. */
const iconed = defineShape({
    name: "iconed",
    parameters: schema.object({ icon: schema.string() }),
    objects: [page],
    queries: ({ icon }) => [page.query({ where: { icon } })],
});

/** Follow the universe's pages titled Plan. */
const plans = Follow.of(titled, { keys: [{ key: { scope: Scope.universe.id, title: "Plan" } }] });

/** Follow the pages of each icon the space's pages show. */
const icons = page.follow(iconed, {
    key: async (row) =>
        row.icon === null ? undefined : { key: { scope: row.scope, icon: row.icon } },
});

test("subscribe to a fixed key and to one key per row the follower has, each toward its scope's publisher, and relist once a row changes the keys", async () => {
    // compose the pages with the follows over the space's database, naming each scope's publisher
    const storage = await TestDatabase.create("sqlite", pageDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const publishers = new Map<string, Publisher>();
    const store = new ObjectStore({
        servers: [implementPackage(pagesPackage, { follows: [plans, icons] })],
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        clock: Clock.system,
        origin: { package: Package.of(pagesPackage) },
        principal: spacePrincipal(spaceId),
        callKey: testCallKey,
        events: await testEvents(storage.database),
        database: storage.database,
        publisher: (scope) => {
            // use one publisher per scope
            const publisher = publishers.get(scope) ?? {
                confirm: async () => {},
                stream: () => (async function* () {})(),
            };
            publishers.set(scope, publisher);

            return publisher;
        },
    });
    const subscriber = present(store.subscriber, "the store's subscriber");

    // create a page with an icon, list the subscriptions, then add a page of another icon
    const create = (title: string, icon: string | null) =>
        store.mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), Date.now()), {
            id: RequestId.create(Date.now()),
            calls: [page.call("create", { spaceId, title, ...(icon === null ? {} : { icon }) })],
        });
    await create("Plan", "📌");
    await create("Notes", null);
    const listed = async () =>
        (await subscriber.subscriptions(Snapshot.live(storage.database))).map(
            ({ subscription, publisher }) => ({
                name: subscription.name,
                publication: subscription.publication,
                scope: subscription.scope,
                target: subscription.target,
                parameters: subscription.parameters,
                isScopePublisher: publisher === publishers.get(subscription.scope),
            }),
        );
    const first = await listed();
    await create("Ideas", "💡");
    const second = await listed();

    // name each replica by its shape, scope and parameters, with the shape's queries as parameters
    const plan = {
        name: `titled:${Scope.universe.id}:Plan`,
        publication: "queries",
        scope: Scope.universe.id,
        target: spaceId,
        parameters: { queries: { "0:page": { object: "page", where: { title: "Plan" } } } },
        isScopePublisher: true,
    };
    const icon = (glyph: string) => ({
        name: `iconed:${spaceId}:${glyph}`,
        publication: "queries",
        scope: spaceId,
        target: spaceId,
        parameters: { queries: { "0:page": { object: "page", where: { icon: glyph } } } },
        isScopePublisher: true,
    });
    // follow the store's own chain beside the follows, every link from the space's own publisher, with the grants at the universe
    const chain = (scope: string) => ({
        name: `chain:${spaceId}`,
        publication: "chain",
        scope,
        target: spaceId,
        parameters: {
            scopes: [spaceId, Scope.universe.id],
            local: scope === spaceId ? [page.typeReference] : [],
            types: [],
            via: [],
            between: [],
        },
        isScopePublisher: scope === spaceId,
    });
    const own = [chain(spaceId), chain(Scope.universe.id)];
    expect({
        first,
        second,
        isWatched: (subscriber.watches ?? []).includes(page.table),
    }).toEqual({
        first: [...own, plan, icon("📌")],
        second: [...own, plan, icon("📌"), icon("💡")],
        isWatched: true,
    });
});

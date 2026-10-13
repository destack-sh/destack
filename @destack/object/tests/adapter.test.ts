import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { Package } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Clock, schema } from "@destack/schema";
import { defineAdapter, definePackage } from "@destack/runtime";
import { Health } from "@destack/runtime/health";
import { Server } from "@destack/runtime/server";
import { testCallKey } from "@destack/runtime/test";
import { testEvents } from "../src/test/index.ts";
import { v7 } from "uuid";
import { type AdapterFetch, ObjectExtension } from "../src/server/index.ts";
import { page, pageDatabase } from "./fixture/page.ts";
import { openSpace, spacePrincipal } from "./fixture/space.ts";
import { bearerUser } from "./fixture/user.ts";

/** The space with the pages. */
const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** The pages with a feed adapter answering below /feed. */
const pages = definePackage({
    objects: { page },
    adapters: { feed: defineAdapter("feed", { path: "/feed" }) },
});

/** Answer a feed request with the path and query it arrived at. */
const feed: AdapterFetch = async (request) => {
    const url = new URL(request.url);

    return Response.json({ path: url.pathname, query: url.search });
};

/** Compose the pages over a space's database, answering the adapters the serve hook supplies. */
async function compose(serve: NonNullable<ObjectExtension["serve"]>) {
    const storage = await TestDatabase.create("sqlite", pageDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const events = await testEvents(storage.database);

    return () =>
        ObjectExtension.compose([{ ...ObjectExtension.of(pages, []), serve }], {
            clock: Clock.system,
            origin: { package: Package.of(pages) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events,
            database: storage.database,
        });
}

test("answer each request at or below an adapter's path by its fetch, below the adapter's path", async () => {
    // serve the pages with their feed
    const composed = await compose(() => ({ adapters: { feed } }));
    const server = Server.start({
        clock: Clock.system,
        ...composed().packages[0],
        scope: spaceId,
        resources: new ResourceContext(),
        health: new Health("pages"),
        drainTimeout: 1000,
        authorizeMachine: async () => {},
        authenticate: bearerUser(pages.id, spaceId),
    });
    onTestFinished(() => server.close());

    // send requests below the feed, at it, and beside it
    const answer = async (path: string) => {
        const response = await server.fetch(new Request(`https://pages.test${path}`));

        return response.status === 200
            ? schema.json().parse(await response.json())
            : response.status;
    };

    expect([
        await answer("/feed/rss?limit=2"),
        await answer("/feed"),
        await answer("/feeds"),
    ]).toEqual([{ path: "/rss", query: "?limit=2" }, { path: "/", query: "" }, 404]);
});

test("refuse a package's unanswered adapter and an answer of no declared adapter", async () => {
    // compose the pages with each wrong answer
    const unanswered = await compose(() => ({}));
    const stray = await compose(() => ({ adapters: { feed, atom: feed } }));

    expect(unanswered).toThrow(new TypeError(`package ${pages.name} answers no adapter feed`));
    expect(stray).toThrow(new TypeError(`package ${pages.name} declares no adapter atom`));
});

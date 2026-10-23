import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { defineAdapter, definePackage } from "@destack/object";
import { testCallKey } from "@destack/object/test";
import { testEvents } from "../src/test/index.ts";
import { v7 } from "uuid";
import {
    type AdapterFetch,
    implementPackage,
    ObjectStore,
    type ObjectPackage,
} from "../src/server/index.ts";
import { page, pageDatabase } from "./fixture/page.ts";
import { openSpace, spacePrincipal } from "./fixture/space.ts";
import { users } from "./fixture/user.ts";
import { Clock } from "@destack/environment";
import { TestBearer, TestObjectStore } from "../src/test/index.ts";
import { principal } from "@destack/access";
import type { Subject } from "@destack/sync";

/** The space with the pages. */
const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** The pages with a feed adapter answering below /feed. */
const pages = definePackage({
    objects: { page },
    adapters: { feed: defineAdapter("feed", { path: "/feed" }) },
});

/** The pages with a feed adapter answering the space's installation alone. */
const customPages = definePackage({
    objects: { page },
    adapters: { feed: defineAdapter("feed", { path: "/feed", callers: "self" }) },
});

/** Answer a feed request with the path and query it arrived at. */
const feed: AdapterFetch = async (request) => {
    const url = new URL(request.url);

    return Response.json({ path: url.pathname, query: url.search });
};

/** The pages as any served package, whose adapters any answers may claim to answer. */
const served: ObjectPackage = pages;

/** Serve the pages over a space's database, answering the given adapters. */
async function servePages(
    adapters: Readonly<Record<string, AdapterFetch>>,
    handle: ObjectPackage = served,
) {
    const storage = await TestDatabase.create(pageDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const events = await testEvents(storage.database);

    return () =>
        new ObjectStore({
            servers: [implementPackage(handle, { adapters })],
            chain: async () => undefined,
            clock: Clock.system,
            origin: { package: Package.of(handle) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events,
            database: storage.database,
        });
}

test("answer each request at or below an adapter's path by its fetch, below the adapter's path", async () => {
    // serve the pages with their feed
    const composed = await servePages({ feed });
    const serving = TestObjectStore.serve(composed());
    onTestFinished(() => serving[Symbol.asyncDispose]());
    const server = serving.server(pages);

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
    // serve the pages with each wrong answer
    const unanswered = await servePages({});
    const stray = await servePages({ feed, atom: feed });

    expect(unanswered).toThrow(
        new TypeError(`package ${pages.name} does not implement adapter feed`),
    );
    expect(stray).toThrow(new TypeError(`package ${pages.name} does not declare adapter atom`));
});

test("answer an adapter declaring its installation's calls as that principal alone", async () => {
    // serve the feed to the space itself, its bearer the space's identifier
    const composed = await servePages({ feed }, customPages);
    const serving = TestObjectStore.serve(composed());
    onTestFinished(() => serving[Symbol.asyncDispose]());
    const server = serving.server(customPages);

    // call the feed as the space, as a user, and as no one
    const status = async (caller: Subject | undefined) => {
        const headers = caller === undefined ? {} : { authorization: TestBearer.header(caller) };
        const response = await server.fetch(new Request("https://pages.test/feed", { headers }));
        await response.body?.cancel();

        return response.status;
    };
    expect([
        await status(spacePrincipal(spaceId)),
        await status(principal.user.reference("universe", users.alice)),
        await status(undefined),
    ]).toEqual([200, 403, 401]);
});

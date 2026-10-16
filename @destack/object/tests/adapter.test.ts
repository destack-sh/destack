import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { Package } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { schema, nth } from "@destack/schema";
import { defineAdapter, definePackage } from "@destack/runtime";
import { Health } from "@destack/runtime/health";
import { Server } from "@destack/runtime/server";
import { testCallKey } from "@destack/runtime/test";
import { testEvents } from "../src/test/index.ts";
import { v7 } from "uuid";
import {
    type AdapterFetch,
    defineServer,
    ObjectStore,
    type ServedPackage,
} from "../src/server/index.ts";
import { page, pageDatabase } from "./fixture/page.ts";
import { openSpace, spacePrincipal } from "./fixture/space.ts";
import { bearerUser } from "./fixture/user.ts";
import { Clock } from "@destack/environment";

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

/** The pages as any served package, whose adapters any answers may claim to answer. */
const served: ServedPackage = pages;

/** Serve the pages over a space's database, answering the given adapters. */
async function servePages(adapters: Readonly<Record<string, AdapterFetch>>) {
    const storage = await TestDatabase.create("sqlite", pageDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const events = await testEvents(storage.database);

    return () =>
        new ObjectStore({
            servers: [defineServer(served, { adapters })],
            chain: async () => undefined,
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
    const composed = await servePages({ feed });
    const server = Server.start({
        clock: Clock.system,
        ...nth(composed().packages, 0),
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
    // serve the pages with each wrong answer
    const unanswered = await servePages({});
    const stray = await servePages({ feed, atom: feed });

    expect(unanswered).toThrow(new TypeError(`package ${pages.name} does not answer adapter feed`));
    expect(stray).toThrow(new TypeError(`package ${pages.name} does not declare adapter atom`));
});

import { Caller } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "@destack/object/client";
import type { ObjectStore } from "@destack/object/server";
import { ResourceContext } from "@destack/resource/context";
import type { ClientOptions } from "@destack/runtime/client";
import { Health } from "@destack/runtime/health";
import { Server } from "@destack/runtime/server";
import { onTestFinished } from "@destack/test";
import type { ViewContext } from "@destack/view/declare";
import { activity, notification } from "../../src/index.ts";
import notificationPackage from "../../src/package.ts";
import { document } from "../fixture/document.ts";
import { actors, serveSpace } from "../fixture/space.ts";
import { Clock } from "@destack/environment";

/** The longest a token the test server issues lives, in milliseconds. */
const TOKEN_LIFETIME = 60_000;

/** A view's context and the clients of the scopes its host opens, the person's home among them. */
export interface Home {
    /** What the host gives the view. */
    readonly context: ViewContext;
    /** The open clients, by scope. */
    readonly clients: Readonly<Record<string, ObjectClient>>;
    /** The failures the device reported while following and pushing. */
    readonly failures: readonly unknown[];
}

/** Serve a space where alice mentions bob twice, and open bob's home on a device. */
export async function openHome(): Promise<Home> {
    // mention bob in two documents at the real time his device stamps its requests with
    const scenario = await serveSpace("sqlite", { at: Date.now() });
    const plan = await scenario.call(
        document.call("create", { ...scenario.scope, title: "Launch plan" }),
    );
    const draft = await scenario.call(
        document.call("create", { ...scenario.scope, title: "Draft" }),
    );
    for (const shared of [plan, draft]) {
        await scenario.call(
            document.call("grant", {
                ...scenario.scope,
                id: shared.id,
                relation: "editor",
                subject: actors.bob,
            }),
        );
        await scenario.call(
            document.call("remark", {
                ...scenario.scope,
                id: shared.id,
                text: "@bob, have a look",
                mentions: [actors.bob],
            }),
        );
    }

    // plan both notifications in his home
    await scenario.dispatch();
    const home = await scenario.home("bob");

    // serve the home over HTTP to bob, then follow it from his device
    const endpoint = serveHome(home.store, home.id);
    const objects = { notification, activity };
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables(objects), {
        isMigrated: true,
    });
    const client = await ObjectClient.open({
        clock: Clock.system,
        database: storage.database,
        objects,
        scope: home.id,
        caller: actors.bob,
        endpoint,
        reconnect: () => endpoint,
    });
    const following = new AbortController();
    const failures: unknown[] = [];
    const report = (error: unknown) => failures.push(error);
    const loops = [client.follow(following.signal, report), client.push(following.signal, report)];
    onTestFinished(async () => {
        following.abort();
        await Promise.all(loops);
        await client.close();
        await storage.close();
    });

    return {
        context: {
            installation: "installation-notes",
            space: scenario.spaceId,
            account: "account-019f5530-8000-7000-8000-0000000000ac",
            view: "notes",
            user: actors.bob,
            home: home.id,
        },
        clients: { [home.id]: client },
        failures,
    };
}

/** Serve the home's inbox over HTTP as bob on the real clock, returning his endpoint. */
function serveHome(objects: ObjectStore, homeId: string): ClientOptions {
    const server = Server.start({
        ...objects.package(notificationPackage),
        clock: Clock.system,
        scope: homeId,
        resources: new ResourceContext(),
        health: new Health("notification"),
        drainTimeout: 1000,
        authorizeMachine: async () => {},
        authenticate: async () => {
            const now = Date.now();

            return {
                caller: Caller.of(actors.bob),
                credential: {
                    kind: "session",
                    id: actors.bob.id,
                    audience: notificationPackage.id,
                    scope: homeId,
                    verifiedAt: now,
                    expiresAt: now + TOKEN_LIFETIME,
                },
            };
        },
    });
    onTestFinished(() => server.close());

    return { url: "https://home.test", fetch: (request: Request) => server.fetch(request) };
}

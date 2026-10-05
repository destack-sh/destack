import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "@destack/object/client";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Authentication } from "@destack/service/authentication";
import type { ClientOptions } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { onTestFinished } from "@destack/test";
import type { ViewContext } from "@destack/view/declare";
import { notification } from "../../src/index.ts";
import { inboxService } from "../../src/service/index.ts";
import { document } from "../fixture/document.ts";
import { actors, serveSpace } from "../fixture/space.ts";

/** The package the home's server authenticates its callers for. */
const AUDIENCE = PackageId.parse("package-019f5530-8000-7000-8000-0000000000ad");

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

/** Serve a space where alice mentions bob in two documents, and open bob's home on a device following it. */
export async function openHome(): Promise<Home> {
    // mention bob in two documents and wait for both notifications in his home
    const scenario = await serveSpace("sqlite");
    const plan = await scenario.call(document, "create", { title: "Launch plan" });
    const draft = await scenario.call(document, "create", { title: "Draft" });
    for (const shared of [plan, draft]) {
        await scenario.call(document, "grant", {
            id: shared.id,
            relation: "editor",
            subject: actors.bob,
        });
        await scenario.call(document, "remark", {
            id: shared.id,
            text: "@bob, have a look",
            mentions: [actors.bob],
        });
    }
    await scenario.inbox("bob").until((state) => state.unread === 2);

    // serve the home over HTTP to bob, then follow it from his device
    const endpoint = serveHome(scenario.server, scenario.homeId);
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables({ notification }), {
        isMigrated: true,
    });
    const client = await ObjectClient.open({
        database: storage.database,
        objects: { notification },
        scope: scenario.homeId,
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
            home: scenario.homeId,
        },
        clients: { [scenario.homeId]: client },
        failures,
    };
}

/** Serve the home's inbox over HTTP on the real clock, authenticating every request as bob, and return bob's endpoint. */
function serveHome(
    objects: Awaited<ReturnType<typeof serveSpace>>["server"],
    homeId: string,
): ClientOptions {
    const server = Server.start({
        ...objects.implement(inboxService),
        clock: () => Date.now(),
        audience: AUDIENCE,
        scope: homeId,
        resources: new ResourceContext(),
        health: new Health(inboxService.name),
        drainTimeout: 1000,
        authorizeHost: async () => {},
        authenticate: async () => {
            const now = Date.now();

            return new Authentication({
                subject: actors.bob,
                subjects: [actors.bob],
                credential: { kind: "user", id: actors.bob.id },
                audience: AUDIENCE,
                scope: homeId,
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME,
            });
        },
    });
    onTestFinished(() => server.close());

    return { url: "https://home.test", fetch: (request: Request) => server.fetch(request) };
}

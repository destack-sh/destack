import { Caller } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient, ObjectConnection } from "@destack/object/client";
import type { ObjectStore } from "@destack/object/server";
import { ResourceContext } from "@destack/resource/context";
import { ObjectConnectionHost, Health, Server } from "@destack/object/server";
import { onTestFinished } from "@destack/test";
import type { ViewContext } from "@destack/view/declare";
import { activity, notification } from "../../src/object/index.ts";
import notificationPackage from "../../src/package.ts";
import { document } from "../../src/test/notes.ts";
import { TestNotification } from "../../src/test/notification.ts";
import { type Environment, WebSocketUpgrade } from "@destack/environment";
import type { Subject } from "@destack/sync";
import {
    BrowserSimulation,
    type SimulatedBrowserEnvironment,
} from "@destack/environment/browser/simulation";

/** The longest a token the test server issues lives, in milliseconds. */
const TOKEN_LIFETIME = 60_000;

/** A view's context and the clients of the scopes its host opens, the person's home among them. */
export interface Home {
    /** What the host gives the view. */
    readonly context: ViewContext;
    /** The open clients, by scope. */
    readonly clients: Readonly<Record<string, ObjectClient>>;
    /** The browser bob's device renders in. */
    readonly environment: SimulatedBrowserEnvironment;
}

/** Serve a space where alice mentions bob twice, and open bob's home on a device. */
export async function openHome(): Promise<Home> {
    // mention bob in two documents
    const scenario = await TestNotification.open();
    onTestFinished(() => scenario[Symbol.asyncDispose]());
    const plan = await scenario.call(
        "alice",
        document.call("create", { ...scenario.scope, title: "Launch plan" }),
    );
    const draft = await scenario.call(
        "alice",
        document.call("create", { ...scenario.scope, title: "Draft" }),
    );
    for (const shared of [plan, draft]) {
        await scenario.call(
            "alice",
            document.call("grant", {
                ...scenario.scope,
                id: shared.id,
                relation: "editor",
                subject: scenario.subject("bob"),
            }),
        );
        await scenario.call(
            "alice",
            document.call("remark", {
                ...scenario.scope,
                id: shared.id,
                text: "@bob, have a look",
                mentions: [scenario.subject("bob")],
            }),
        );
    }

    // plan both notifications in his home
    await scenario.simulation.idle();
    const home = await scenario.home("bob");

    // serve the home to bob, then follow it from his device over one connection
    const connection = serveHome(
        await home.space.store(),
        home.space.id,
        scenario.subject("bob"),
        scenario.environment,
    );
    const objects = { notification, activity };
    const storage = await TestDatabase.create(ObjectClient.tables(objects), {
        dialect: "sqlite",
        isMigrated: true,
        environment: scenario.environment,
    });
    const client = await ObjectClient.open({
        environment: scenario.environment,
        database: storage.database,
        objects,
        scope: home.space.id,
        caller: scenario.subject("bob"),
        remote: connection,
    });
    const following = new AbortController();
    const loops = [client.follow(following.signal), client.push(following.signal)];
    onTestFinished(async () => {
        following.abort();
        await Promise.all(loops);
        await client.close();
        await storage.close();
    });

    return {
        context: {
            installation: "installation-notes",
            space: scenario.scope.spaceId,
            account: "account-019f5530-8000-7000-8000-0000000000ac",
            view: "notes",
            universe: "https://destack.test",
            catalogs: [],
            base: "",
            user: scenario.subject("bob"),
            home: home.space.id,
            residencyHome: home.space.id,
        },
        clients: { [home.space.id]: client },
        environment: new BrowserSimulation(scenario.space.simulation).environment("bob"),
    };
}

/** Serve the home's inbox over HTTP as bob in the scenario's world, returning his endpoint. */
function serveHome(
    objects: ObjectStore,
    homeId: string,
    person: Subject,
    environment: Environment,
): ObjectConnection {
    const sockets = new ObjectConnectionHost(environment.telemetry);
    const server = Server.start({
        ...objects.package(notificationPackage),
        environment,
        hibernation: sockets.hibernation,
        scopes: [homeId],
        resources: new ResourceContext(),
        health: new Health("notification"),
        drainTimeout: 1000,
        authorizeMachine: async () => {},
        authenticate: async () => {
            const now = environment.clock.now();

            return {
                caller: Caller.of(person),
                credential: {
                    kind: "session",
                    id: person.id,
                    audience: notificationPackage.id,
                    scope: homeId,
                    verifiedAt: now,
                    expiresAt: now + TOKEN_LIFETIME,
                },
            };
        },
    });
    onTestFinished(() => server.close());
    sockets.serve(notificationPackage.id, server);

    return ObjectConnection.open({
        scope: homeId,
        url: ObjectConnection.url("https://home.test"),
        websocket: (url, protocols) =>
            sockets.open((upgrade) => server.fetch(upgrade), WebSocketUpgrade.to(url, protocols)),
        environment,
    });
}

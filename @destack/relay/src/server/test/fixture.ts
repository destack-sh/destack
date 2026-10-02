import { Snapshot } from "@destack/db";
import { Scope } from "@destack/sync";
import { createServer } from "node:net";
import { serve } from "bun";
import type { HostIdentity } from "@destack/host/identity";
import { type GlobalFixture, ACCOUNTS_URL, ids, ISSUER } from "@destack/host/test";
import { TokenVerifier } from "@destack/service/authentication";
import { Resolver } from "@destack/account/directory";
import { DirectoryStore } from "@destack/directory";
import { identifier, schema, type Identifier } from "@destack/schema";
import { space } from "@destack/space/object";
import { v7 } from "uuid";
import { TunnelClient, type TunnelClientOptions } from "../../tunnel/index.ts";
import { RelayServer, type RelayServerOptions } from "../../bun/index.ts";
import { RELAY_PACKAGE } from "../index.ts";

/** Timings short enough for scenarios to watch renewals and reconnects. */
export const QUICK = {
    heartbeat: 100,
    retry: { initialInterval: 20, maximumInterval: 40 },
} as const;

/** The code of a refusal's JSON body. */
const Refusal = schema.looseObject({ code: schema.string() });

/** An enrolled host serving a space, and the relays reaching it. */
export class RelayFixture implements AsyncDisposable {
    /** The global tier. */
    readonly global: GlobalFixture;
    /** The directory over the global database. */
    readonly directory: DirectoryStore;
    /** The host serving the space. */
    readonly identity: HostIdentity;
    /** The host's identifier. */
    readonly hostId: Identifier<"host">;
    /** The name of the host's space. */
    readonly space: string;
    /** The host's space. */
    readonly spaceId: Identifier<"space">;
    /** The requests the host answered, as method and URL. */
    readonly received: string[] = [];
    /** The failures the relays and tunnels reported. */
    readonly reports: unknown[] = [];
    /** The relays started, closed on disposal. */
    readonly #relays = new Set<RelayServer>();
    /** The clients and region servers started, closed on disposal. */
    readonly #hosts: { close(): Promise<void> }[] = [];

    /** Keep a prepared global tier. */
    private constructor(
        global: GlobalFixture,
        identity: HostIdentity,
        name: string,
        spaceId: Identifier<"space">,
    ) {
        // keep the tier, its directory, the host and its space
        this.global = global;
        this.directory = new DirectoryStore(global.database);
        this.identity = identity;
        this.hostId = identifier("host").parse(identity.hostId);
        this.space = name;
        this.spaceId = spaceId;
    }

    /** Enroll a new host of the acme account in a global tier, and place a new space in it. */
    static async open(global: GlobalFixture): Promise<RelayFixture> {
        // enroll the host and place its space in it
        const identity = await global.enroll(ids.account);
        const name = `space-${v7().slice(-12)}`;
        const cell = identifier("host").parse(identity.hostId);
        const spaceId = await RelayFixture.place(global, name, cell);

        return new RelayFixture(global, identity, name, spaceId);
    }

    /** The name of the notes app of the host's space. */
    get notes(): string {
        return `notes.${this.space}.acme.destack.space`;
    }

    /** Read the name of the host itself. */
    async computer(): Promise<string> {
        const { name } = await this.global
            .host(this.identity)
            .host.get({ accountId: ids.account, id: this.hostId });

        return `${name}.acme.destack.computer`;
    }

    /** Place the host's space under another name, releasing its current one. */
    async rename(name: string): Promise<void> {
        const row = { id: this.spaceId, scope: ids.account, name };
        await this.directory.replace(
            await space.owned(this.spaceId, row, Snapshot.live(this.global.database)),
            `rename-${this.spaceId}`,
        );
    }

    /** Name a space of the acme account, and place its zone in a host or region. */
    static async place(
        global: GlobalFixture,
        name: string,
        cell: string,
    ): Promise<Identifier<"space">> {
        // claim the name and place the zone
        const directory = new DirectoryStore(global.database);
        const id = identifier("space").parse(`space-${v7()}`);
        const row = { id, scope: ids.account, name };
        await directory.replace(
            await space.owned(id, row, Snapshot.live(global.database)),
            `place-${id}`,
        );
        await directory.place({ id, scope: ids.account, cell, epoch: 1 });

        return id;
    }

    /** Start a relay listening on a port. */
    relay(port: number, options: Partial<RelayServerOptions> = {}): RelayServer {
        const relay = RelayServer.start({
            origin: `http://127.0.0.1:${port}`,
            listener: { hostname: "127.0.0.1", port },
            database: this.global.database,
            resolver: Resolver.global(this.global.database),
            tokens: new TokenVerifier({
                authority: { kind: "universe" },
                issuer: ISSUER,
                audience: RELAY_PACKAGE.id,
                keys: this.global.keys,
            }),
            report: (error) => this.reports.push(error),
            ...options,
        });
        this.#relays.add(relay);

        return relay;
    }

    /** Close a relay before the scenario ends. */
    async stop(relay: RelayServer): Promise<void> {
        this.#relays.delete(relay);
        await relay.close();
    }

    /** Keep a tunnel from the host to a relay, greeting each request with its method, path and body. */
    async tunnel(
        relay: RelayServer,
        options: Partial<TunnelClientOptions> = {},
    ): Promise<TunnelClient> {
        // dial with the host's relay tokens, recording and greeting each request
        const client = TunnelClient.open({
            url: relay.url,
            token: async () => {
                const granted = await this.identity.token(
                    RELAY_PACKAGE.id,
                    ACCOUNTS_URL,
                    (request) => this.global.accounts.fetch(request),
                );

                return granted.accessToken;
            },
            fetch: async (request) => {
                // record the request, and greet it with its method, path and body
                const url = new URL(request.url);
                this.received.push(`${request.method} ${url.host}${url.pathname}`);
                const body = request.body === null ? "" : ` ${await request.text()}`;

                return new Response(`hello ${request.method} ${url.pathname}${body}`);
            },
            heartbeat: QUICK.heartbeat,
            retry: QUICK.retry,
            report: (error) => this.reports.push(error),
            ...options,
        });
        this.#hosts.push(client);
        await client.opened();

        return client;
    }

    /** Serve some spaces' apps as a region answering its name and the name asked for, and publish its endpoint. */
    async region(name: string): Promise<void> {
        const server = serve({
            hostname: "127.0.0.1",
            port: 0,
            fetch: (request) => new Response(`${name} ${request.headers.get("forwarded")}`),
        });
        this.#hosts.push({ close: () => server.stop(true) });
        await this.directory.publish(
            ids.region,
            Scope.universe.id,
            `http://127.0.0.1:${server.port}/`,
        );
    }

    /** Request a path of a name through a relay, returning the body, or the status and code of a refusal. */
    async request(
        relay: RelayServer,
        name: string,
        path: string,
        request: RequestInit = {},
    ): Promise<string> {
        // ask the relay's listener for the name
        const headers = new Headers(request.headers);
        headers.set("host", name);
        const response = await fetch(`http://127.0.0.1:${relay.port}${path}`, {
            ...request,
            headers,
        });

        // answer the body, or the refusal's status and code
        if (response.ok) {
            return response.text();
        }
        const { code } = Refusal.parse(await response.json());

        return `${response.status} ${code}`;
    }

    /** Close every tunnel and relay. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const host of this.#hosts) {
            await host.close();
        }
        for (const relay of this.#relays) {
            await relay.close();
        }
    }
}

/** Find a free local port. */
export async function freePort(): Promise<number> {
    // bind any port, and release it
    const probe = createServer();
    await new Promise<void>((resolve) => {
        probe.listen(0, "127.0.0.1", resolve);
    });
    const address = probe.address();
    await new Promise<void>((resolve) => {
        probe.close(() => resolve());
    });
    if (address === null || typeof address === "string") {
        throw new TypeError("the probe bound no TCP port");
    }

    return address.port;
}

/** Wait until a condition is true, polling it every few milliseconds for at most two seconds. */
export async function until(condition: () => Promise<boolean> | boolean): Promise<void> {
    const deadline = Date.now() + 2000;
    while (!(await condition())) {
        if (Date.now() > deadline) {
            throw new Error("condition was not true within two seconds");
        }
        await new Promise((resolve) => {
            setTimeout(resolve, 10);
        });
    }
}

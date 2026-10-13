import type { MachineIdentity } from "@destack/account/client";
import { space } from "@destack/account/object";
import {
    type AccountFixture,
    ACCOUNTS_URL,
    ids,
    ISSUER,
    TestDirectory,
} from "@destack/account/test";
import { AuditCall } from "@destack/audit";
import { Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Claim, Claimant } from "@destack/object";
import { Identifier, present, schema } from "@destack/schema";
import { TokenVerifier } from "@destack/service/authentication";
import { testCallKey } from "@destack/service/test";
import type { Subject } from "@destack/sync";
import { serve } from "bun";
import { createServer } from "node:net";
import {
    type DestinationCache,
    Relay,
    RELAY_PACKAGE,
    RELAY_TOKEN_LIFETIME_MILLISECONDS,
} from "../server/index.ts";
import { homeDatabase } from "./database.ts";
import { TunnelClient, type TunnelClientOptions } from "../tunnel/index.ts";
import { RelayHost } from "./host.ts";

/** The longest a scenario waits for a relay's copies, in milliseconds. */
const SETTLE_MILLISECONDS = 10_000;

/** The longest a scenario polls a condition, in milliseconds. */
const UNTIL_MILLISECONDS = 2000;

/** How often a scenario polls a condition, in milliseconds. */
const POLL_MILLISECONDS = 10;

/** Timings short enough for scenarios to watch renewals and reconnects. */
const QUICK = {
    heartbeat: 100,
    retry: { initialInterval: 20, maximumInterval: 40 },
} as const;

/** The code of a refusal's JSON body. */
const Refusal = schema.looseObject({ code: schema.string() });

/** An enrolled machine serving a space, and the relays routing to it. */
export class RelayFixture implements AsyncDisposable {
    /** The account service. */
    readonly accounts: AccountFixture;
    /** The relay installation in the residency's home space. */
    readonly installation: Subject;
    /** The machine serving the space. */
    readonly identity: MachineIdentity;
    /** The machine's identifier. */
    readonly machineId: Identifier<"machine">;
    /** The name of the machine's space. */
    readonly space: string;
    /** The machine's space. */
    readonly spaceId: Identifier<"space">;
    /** The requests the machine answered, as method and URL. */
    readonly received: string[] = [];
    /** The names the relays told the machine they route to it, in order. */
    readonly names: string[] = [];
    /** The audited calls the relays' journals delivered, in order. */
    readonly audited: AuditCall[] = [];
    /** The failures the relays and tunnels reported. */
    readonly reports: unknown[] = [];
    /** How far the relays' clock runs ahead of the system's, in milliseconds. */
    skew = 0;
    /** The relays started with their services and servers, closed on disposal. */
    readonly #relays = new Map<RelayHost, { readonly home: string }>();
    /** The relays' databases, closed on disposal. */
    readonly #databases: TestDatabase[] = [];
    /** The tunnels and cloud machine servers started, closed on disposal. */
    readonly #machines: { close(): Promise<void> }[] = [];

    /** Keep a prepared account service. */
    private constructor(
        accounts: AccountFixture,
        installation: Subject,
        identity: MachineIdentity,
        name: string,
        spaceId: Identifier<"space">,
    ) {
        // keep the account service with the relay installation
        this.accounts = accounts;
        this.installation = installation;

        // keep the machine and its space
        this.identity = identity;
        this.machineId = schema.identifier("machine").parse(identity.machineId);
        this.space = name;
        this.spaceId = spaceId;
    }

    /** Enroll a new personal machine of an account in an account service. */
    static async open(
        accounts: AccountFixture,
        installation: Subject,
        accountId: Identifier<"account"> = ids.account,
    ): Promise<RelayFixture> {
        // enroll the machine and place its space in it
        const identity = await accounts.enroll(accountId);
        const name = `space-${Identifier.create("space").slice(-12)}`;
        const spaceId = await RelayFixture.place(accounts, name, identity.machineId, accountId);

        return new RelayFixture(accounts, installation, identity, name, spaceId);
    }

    /** The name of the notes app of the machine's space. */
    get notes(): string {
        return `notes--${this.space}--acme.destack.space`;
    }

    /** Read the name of the machine itself. */
    async computer(): Promise<string> {
        const { name } = await this.accounts
            .machine(this.identity)
            .machine.get({ scope: ids.account, id: this.machineId });

        return `${name}--acme.destack.computer`;
    }

    /** Place the machine's space under another name, releasing its current one. */
    async rename(name: string): Promise<void> {
        await RelayFixture.#claim(this.accounts, { id: this.spaceId, scope: ids.account, name });
    }

    /** Create a space of an account, the acme account by default, and place it on a machine. */
    static async place(
        accounts: AccountFixture,
        name: string,
        machine: Identifier<"machine">,
        accountId: Identifier<"account"> = ids.account,
    ): Promise<Identifier<"space">> {
        // claim the name and place the space
        const id = Identifier.create("space");
        await RelayFixture.#claim(accounts, { id, scope: accountId, name });
        await TestDirectory.place(accounts.database, { id, scope: accountId, machine, epoch: 1 });

        return id;
    }

    /** Claim a space's name in the account service, releasing its earlier one. */
    static async #claim(
        accounts: AccountFixture,
        row: { readonly id: Identifier<"space">; readonly scope: string; readonly name: string },
    ): Promise<void> {
        const snapshot = Snapshot.live(accounts.database);
        const object = { objectId: row.id, owner: row.scope };
        await Claim.replace(accounts.database, await Claimant.of(space, object, row, snapshot));
    }

    /** Start a relay installation, the fixture's unless given, on a port over a home's database. */
    async relay(
        port: number,
        options: {
            readonly destinations?: DestinationCache;
            readonly installation?: Subject;
        } = {},
    ): Promise<RelayHost> {
        // keep the relay's rows and copies in a home's main database
        const { destinations, installation = this.installation } = options;
        const storage = await TestDatabase.create("sqlite", homeDatabase, { isMigrated: true });
        this.#databases.push(storage);

        // follow and call the account service as the relay installation of its residency's home space, served as a machine hosts it
        const host = RelayHost.start(port, installation, {
            database: storage.database,
            accounts: this.accounts.installation(installation),
            tokens: new TokenVerifier({
                authority: { kind: "universe" },
                issuer: ISSUER,
                audience: RELAY_PACKAGE.id,
                lifetime: RELAY_TOKEN_LIFETIME_MILLISECONDS,
                keys: this.accounts.keys,
            }),
            ...(destinations === undefined ? {} : { destinations }),
            callKey: testCallKey,
            history: {
                ingest: async (batch) => {
                    this.audited.push(...batch.calls.map((call) => AuditCall.parse(call)));
                },
            },
            report: (error) => this.reports.push(error),
            clock: { now: () => Date.now() + this.skew },
        });
        this.#relays.set(host, { home: installation.scope });
        await this.settle(host);

        return host;
    }

    /** Wait until a relay's copies reflect the account service as of now. */
    async settle(relay: RelayHost): Promise<void> {
        // wait for the copies of the residency's accounts
        const { home } = present(this.#relays.get(relay), "the relay's host");
        const { objects } = relay.service;
        await this.accounts.settle(objects, home);

        // wait for the copy of the directory
        const head = await this.accounts.database.log.position(await objects.database.log.epoch());
        const copy = objects.source.replicaOf(Relay.subscription(home));
        const signal = AbortSignal.timeout(SETTLE_MILLISECONDS);
        if (!(await copy.wait(objects.database, head, signal))) {
            throw new Error("the relay's copy of the directory did not catch up");
        }
    }

    /** Close a relay before the scenario ends. */
    async stop(relay: RelayHost): Promise<void> {
        // forget the relay and close it
        this.#relays.delete(relay);
        await relay.close();
    }

    /** Read a token of the machine for its relays. */
    async token(): Promise<string> {
        const granted = await this.identity.token(RELAY_PACKAGE.id, ACCOUNTS_URL, (request) =>
            this.accounts.fetch(request),
        );

        return granted.accessToken;
    }

    /** Keep a tunnel from the machine to a relay. */
    async tunnel(
        relay: RelayHost,
        options: Partial<TunnelClientOptions> = {},
    ): Promise<TunnelClient> {
        // dial with the machine's relay tokens, recording and greeting each request
        const client = TunnelClient.open({
            url: relay.url,
            token: () => this.token(),
            fetch: async (request) => {
                // record the request, and greet it with its method, path and body
                const url = new URL(request.url);
                this.received.push(`${request.method} ${url.host}${url.pathname}`);
                const body = request.body === null ? "" : ` ${await request.text()}`;

                return new Response(`hello ${request.method} ${url.pathname}${body}`);
            },
            name: (name) => this.names.push(name),
            heartbeat: QUICK.heartbeat,
            retry: QUICK.retry,
            report: (error) => this.reports.push(error),
            ...options,
        });
        this.#machines.push(client);
        await client.opened();

        return client;
    }

    /** Serve some spaces' apps as a cloud machine of the platform, eu unless given. */
    async cloud(name: string, machine: Identifier<"machine"> = ids.machine): Promise<void> {
        const server = serve({
            hostname: "127.0.0.1",
            port: 0,
            fetch: (request) => new Response(`${name} ${request.headers.get("forwarded")}`),
        });
        this.#machines.push({ close: () => server.stop(true) });
        await TestDirectory.publish(
            this.accounts.database,
            machine,
            this.accounts.platform,
            `http://127.0.0.1:${server.port}/`,
        );
    }

    /** Request a path of a name through a relay. */
    async request(
        relay: RelayHost,
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
        for (const started of this.#machines) {
            await started.close();
        }
        for (const relay of this.#relays.keys()) {
            await relay.close();
        }
        for (const storage of this.#databases) {
            await storage.close();
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
    const deadline = Date.now() + UNTIL_MILLISECONDS;
    while (!(await condition())) {
        if (Date.now() > deadline) {
            throw new Error(`condition was not true within ${UNTIL_MILLISECONDS} ms`);
        }
        await new Promise((resolve) => {
            setTimeout(resolve, POLL_MILLISECONDS);
        });
    }
}

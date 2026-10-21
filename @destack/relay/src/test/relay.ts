import relayPackage from "../package.ts";
import { telemetry } from "@destack/telemetry";
import { TestTelemetry } from "@destack/telemetry/test";
import type { MachineIdentity } from "@destack/account/client";
import { endpoint, type Endpoint, space } from "@destack/account/object";
import { type AccountFixture, ids, ISSUER, TestDirectory } from "@destack/account/test";
import { journal, type JournalEntry, Claim, Claimant } from "@destack/object";
import { asc, eq, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Identifier, schema } from "@destack/schema";
import { RequestId, replicaPosition, type Subject } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { testEvents } from "@destack/object/test";
import { RELAY_TOKEN_LIFETIME_MILLISECONDS } from "../server/index.ts";
import { homeDatabase, tunnelDatabase } from "./database.ts";
import { TunnelClient, type TunnelClientOptions } from "../tunnel/index.ts";
import type { Renewal } from "../session/index.ts";
import { RelayHost } from "./host.ts";
import type { SimulatedNetwork, Simulation, VirtualClock } from "@destack/environment/simulation";

/** The instruments capturing the failed publications of the scenario machine. */
const instruments = telemetry.scope(import.meta.destack.package);

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
    /** The account package. */
    readonly accounts: AccountFixture;
    /** The relay installation in the residency's home space. */
    readonly installation: Subject;
    /** The machine serving the space. */
    readonly identity: MachineIdentity;
    /** The machine's identifier. */
    readonly machineId: Identifier<"machine">;
    /** The account the machine belongs to. */
    readonly accountId: Identifier<"account">;
    /** The name of the machine's space. */
    readonly space: string;
    /** The machine's space. */
    readonly spaceId: Identifier<"space">;
    /** The requests the machine answered, as method and URL. */
    readonly received: string[] = [];
    /** The names the relays told the machine they route to it, in order. */
    readonly names: string[] = [];
    /** The telemetry recording the exceptions the relays, tunnels and machine captured. */
    readonly telemetry = TestTelemetry.register();
    /** The simulation the account package, the relays and the machine run in. */
    readonly simulation: Simulation;
    /** The relays started with their stores and servers, closed on disposal. */
    readonly #relays = new Set<RelayHost>();
    /** The relays' databases, closed on disposal. */
    readonly #databases: TestDatabase[] = [];
    /** The tunnels and cloud machine servers started, closed on disposal. */
    readonly #machines: { readonly close: () => Promise<void> }[] = [];

    /** Create the fixture over a prepared account package. */
    private constructor(
        simulation: Simulation,
        accounts: AccountFixture,
        installation: Subject,
        identity: MachineIdentity,
        accountId: Identifier<"account">,
        name: string,
        spaceId: Identifier<"space">,
    ) {
        // set the simulation, the account package and the relay installation
        this.simulation = simulation;
        this.accounts = accounts;
        this.installation = installation;

        // set the machine and its space
        this.identity = identity;
        this.machineId = schema.identifier("machine").parse(identity.machineId);
        this.accountId = accountId;
        this.space = name;
        this.spaceId = spaceId;
    }

    /** Enroll a new personal machine of an account in the account package. */
    static async open(
        simulation: Simulation,
        accounts: AccountFixture,
        installation: Subject,
        accountId: Identifier<"account"> = ids.account,
    ): Promise<RelayFixture> {
        // enroll the machine and place its space in it
        const identity = await accounts.enroll(accountId);
        const name = `space-${Identifier.create("space").slice(-12)}`;
        const spaceId = await RelayFixture.place(accounts, name, identity.machineId, accountId);

        return new RelayFixture(
            simulation,
            accounts,
            installation,
            identity,
            accountId,
            name,
            spaceId,
        );
    }

    /** The clock the account package, the relays and the machine run on. */
    get clock(): VirtualClock {
        return this.simulation.clock;
    }

    /** The network between the relays, the machine, the cloud machines and their callers. */
    get network(): SimulatedNetwork {
        return this.simulation.network;
    }

    /** Run the simulation until a condition holds, within a relay token's lifetime of its time. */
    async until(condition: () => Promise<boolean> | boolean): Promise<void> {
        await this.simulation.run({
            until: condition,
            for: { milliseconds: RELAY_TOKEN_LIFETIME_MILLISECONDS },
        });
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
        machineId: Identifier<"machine">,
        accountId: Identifier<"account"> = ids.account,
    ): Promise<Identifier<"space">> {
        // claim the name and place the space
        const id = Identifier.create("space");
        await RelayFixture.#claim(accounts, { id, scope: accountId, name });
        await TestDirectory.place(accounts.database, {
            id,
            scope: accountId,
            machine: machineId,
            epoch: 1,
        });

        return id;
    }

    /** Claim a space's name in the account package, releasing its earlier one. */
    static async #claim(
        accounts: AccountFixture,
        row: { readonly id: Identifier<"space">; readonly scope: string; readonly name: string },
    ): Promise<void> {
        const snapshot = Snapshot.live(accounts.database);
        const object = { objectId: row.id, owner: row.scope };
        await Claim.replace(accounts.database, await Claimant.of(space, object, row, snapshot));
    }

    /** Start a relay installation, the fixture's unless given, over a home's database. */
    async relay(
        options: {
            readonly installation?: Subject;
            readonly port?: number;
        } = {},
    ): Promise<RelayHost> {
        // store the relay's rows and replicas in a home's main database
        const { installation = this.installation, port = 0 } = options;
        const storage = await TestDatabase.create("sqlite", homeDatabase, { isMigrated: true });
        this.#databases.push(storage);

        // insert the home space's row, as the space's own store stores it where the space is placed
        const universe = Snapshot.live(this.accounts.database);
        await RelayHost.home(storage.database, universe, installation);

        // follow and call the account package as its residency home's relay, served as a machine hosts it
        const host = RelayHost.start(installation, {
            environment: this.simulation.environment(`relay-${this.#relays.size}`),
            port,
            database: storage.database,
            events: await testEvents(storage.database),
            accounts: this.accounts.installation(installation),
            issuer: ISSUER,
            callKey: testCallKey,
            // open each tunnel's partition over its own database, as a machine opens its object
            partitions: async () => {
                const partition = await TestDatabase.create("sqlite", tunnelDatabase, {
                    isMigrated: true,
                });
                this.#databases.push(partition);

                return {
                    database: partition.database,
                    events: await testEvents(partition.database),
                };
            },
        });
        this.#relays.add(host);
        await this.settle(host);

        return host;
    }

    /** Wait until a relay's replicas reflect the account package as of now. */
    async settle(relay: RelayHost): Promise<void> {
        await this.accounts.settle(relay.implementation.objects);
    }

    /** Run the clock ahead, letting each relay's follows confirm their replicas at heartbeats. */
    async advance(milliseconds: number): Promise<void> {
        // move the clock past the heartbeats on the way
        this.clock.advance(milliseconds);
        const now = this.clock.now();

        // run until each relay's replicas, its partitions' among them, are confirmed at the new time
        const isConfirmed = async (storage: TestDatabase) => {
            const rows = await storage.database
                .select({ confirmedAt: replicaPosition.confirmedAt })
                .from(replicaPosition);

            return rows.every((row) => row.confirmedAt >= now);
        };
        await this.until(async () =>
            (await Promise.all(this.#databases.map(isConfirmed))).every(Boolean),
        );
    }

    /** List the identifiers of the connections a relay records. */
    async connections(relay: RelayHost): Promise<string[]> {
        return relay.connections();
    }

    /** Read the calls the relays journaled with their inputs, each relay's in start order. */
    async journaled(): Promise<JournalEntry[]> {
        const calls: JournalEntry[] = [];
        for (const storage of this.#databases) {
            const rows = await storage.database
                .select({ call: journal.call })
                .from(journal)
                .orderBy(asc(journal.startedAt), asc(journal.id));
            calls.push(...rows.map((row) => row.call));
        }

        return calls;
    }

    /** Close a relay before the scenario ends. */
    async stop(relay: RelayHost): Promise<void> {
        // forget the relay and close it
        this.#relays.delete(relay);
        await relay.close();
    }

    /** Read a token of the machine for its relays. */
    async token(): Promise<string> {
        return this.identity.token(relayPackage.id, { url: ISSUER }, (request) =>
            this.accounts.fetch(request),
        );
    }

    /** Read the machine's endpoint as the account package stores it, null before it publishes. */
    async endpoint(): Promise<Endpoint | null> {
        const [row] = await this.accounts.database
            .select()
            .from(endpoint.table)
            .where(eq(endpoint.table.id, this.machineId));

        return row ?? null;
    }

    /** Open a tunnel from the machine to a relay, publishing its endpoint at each renewal. */
    async tunnel(
        relay: RelayHost,
        options: Partial<TunnelClientOptions> = {},
    ): Promise<TunnelClient> {
        // record the name and publish the endpoint as the machine, unless the scenario renews itself
        const { renewed = (renewal) => this.#publish(relay, renewal), ...rest } = options;
        const first = Promise.withResolvers<void>();
        const renew = (renewal: Renewal) => {
            Promise.resolve(renewed(renewal)).then(
                () => first.resolve(),
                (error: unknown) => instruments.captureException(error),
            );
        };

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
            renewed: renew,
            heartbeat: QUICK.heartbeat,
            retry: QUICK.retry,
            environment: this.simulation.environment("machine"),
            ...rest,
        });
        this.#machines.push(client);

        // return once the tunnel opened and the machine handled its first renewal
        await client.opened();
        await first.promise;

        return client;
    }

    /** Record a renewal's name and publish the machine's endpoint at it. */
    async #publish(relay: RelayHost, renewal: Renewal): Promise<void> {
        this.names.push(renewal.name);
        await this.accounts.machine(this.identity).machine.publish({
            scope: this.accountId,
            id: this.machineId,
            requestId: RequestId.create(Date.now()),
            url: `https://${renewal.name}`,
            relay: { url: new URL(relay.url).origin, lapsesAt: renewal.lapsesAt },
        });
    }

    /** Serve some spaces' apps as a cloud machine of the platform, eu unless given. */
    async cloud(name: string, machineId: Identifier<"machine"> = ids.machine): Promise<void> {
        const listener = this.network.of(`cloud-${name}`).serve({
            hostname: "127.0.0.1",
            port: 0,
            fetch: (request) => new Response(`${name} ${request.headers.get("forwarded")}`),
        });
        this.#machines.push({ close: () => listener.stop(true) });
        await TestDirectory.publish(
            this.accounts.database,
            machineId,
            this.accounts.platform,
            listener.url.href,
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
        const response = await this.network
            .of("caller")
            .fetch(new Request(`http://127.0.0.1:${relay.port}${path}`, { ...request, headers }));

        // answer the body, or the refusal's status and code
        if (response.ok) {
            return response.text();
        }
        const { code } = Refusal.parse(await response.json());

        return `${response.status} ${code}`;
    }

    /** Refuse exceptions the scenario left untaken, then close every tunnel, relay and database. */
    async [Symbol.asyncDispose](): Promise<void> {
        try {
            // refuse the exceptions before the closes add their own
            this.telemetry[Symbol.dispose]();
        } finally {
            // close the machines' tunnels, the relays and their databases
            for (const started of this.#machines) {
                await started.close();
            }
            for (const relay of this.#relays) {
                await relay.close();
            }
            for (const storage of this.#databases) {
                await storage.close();
            }
        }
    }
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

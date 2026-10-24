import { testSimulation } from "@destack/test/world";
import { Failure } from "@destack/error";
import type { Environment } from "@destack/environment";
import { endpoint, type Endpoint, type ResidencyCode } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import type { TestPerson } from "@destack/account/test";
import { Snapshot } from "@destack/db";
import { Simulation, type SimulatedClock } from "@destack/environment/simulation";
import { createClient } from "@destack/object/client";
import { Identifier } from "@destack/schema";
import { RequestId, Scope, type Subject } from "@destack/sync";
import { TelemetryScope } from "@destack/telemetry";
import { TestTelemetry } from "@destack/telemetry/test";
import { type TestMachine, TestUniverse } from "@destack/universe/test";
import relayPackage from "../package.ts";
import { TUNNEL_TOKEN_LIFETIME_MILLISECONDS } from "../server/index.ts";
import type { Renewal } from "../session/index.ts";
import { TunnelClient, type TunnelClientOptions } from "../tunnel/index.ts";
import { TestRelay } from "./relay.ts";

/** The residencies of the universe, each with its cloud machine. */
const RESIDENCIES = [
    { code: "eu", name: "Europe" },
    { code: "us", name: "United States" },
] as const;

/** The handle of the account the computer belongs to. */
const HANDLE = "acme";

/** The name of the space the computer serves. */
const SPACE = "work";

/** The origin the computer publishes as it enrolls, where nothing listens. */
const COMPUTER_ORIGIN = "https://computer.test/";

/** Timings short enough for scenarios to watch renewals and reconnects. */
const QUICK = {
    heartbeat: 100,
    retry: { initialInterval: 20, maximumInterval: 40 },
} as const;

/** What a relay answers a request: its body, or its refusal's code, reason and message. */
export type RelayAnswer = string | readonly [string, string, string];

/** A person's computer serving a space of their account through relays, in a simulated universe. */
export class TestTunnelMachine implements AsyncDisposable {
    /** The simulation the universe, the relays and the computer run in. */
    readonly simulation: Simulation;
    /** The universe whose names the relays route. */
    readonly universe: TestUniverse;
    /** The owner of the computer's account. */
    readonly owner: TestPerson;
    /** The computer, enrolled as a personal machine of the account. */
    readonly machine: TestMachine;
    /** The account the computer belongs to. */
    readonly accountId: Identifier<"account">;
    /** The space the computer serves. */
    readonly spaceId: Identifier<"space">;
    /** The requests the computer answered, as method and URL. */
    readonly received: string[] = [];
    /** The headers of each request the computer answered, in order. */
    readonly heads: (readonly [string, string])[][] = [];
    /** The names the relays told the computer they route to it, in order. */
    readonly names: string[] = [];
    /** The telemetry recording the exceptions the relays, tunnels and computer captured. */
    readonly telemetry: TestTelemetry;
    /** The cloud machine of each residency opened so far. */
    readonly #hosts = new Map<ResidencyCode, TestMachine>();
    /** The relays, tunnels and listeners started, stopped in reverse on disposal. */
    readonly #started = new AsyncDisposableStack();
    /** The relays started so far. */
    #relays = 0;

    /** The world the computer runs in, as its node. */
    get environment(): Environment {
        return this.simulation.environment("computer");
    }

    /** Wrap an enrolled computer and the space it serves. */
    private constructor(fields: {
        readonly simulation: Simulation;
        readonly universe: TestUniverse;
        readonly owner: TestPerson;
        readonly machine: TestMachine;
        readonly accountId: Identifier<"account">;
        readonly spaceId: Identifier<"space">;
    }) {
        // take the simulation, the universe and the owner
        this.simulation = fields.simulation;
        this.telemetry = new TestTelemetry(fields.simulation.telemetry);
        this.universe = fields.universe;
        this.owner = fields.owner;

        // take the computer, its account and its space
        this.machine = fields.machine;
        this.accountId = fields.accountId;
        this.spaceId = fields.spaceId;
    }

    /** Enroll a computer of the acme account in a residency, Europe by default. */
    static async open(residency: ResidencyCode = "eu"): Promise<TestTunnelMachine> {
        // run the universe in a simulation, driving its opening
        const simulation = testSimulation();

        return await simulation.drive(TestTunnelMachine.#open(simulation, residency));
    }

    /** Open the universe on a simulation, granting relay tokens, and enroll the computer. */
    static async #open(
        simulation: Simulation,
        residency: ResidencyCode,
    ): Promise<TestTunnelMachine> {
        // open the universe, closed again should the enrollment fail
        await using opening = new AsyncDisposableStack();
        const universe = await TestUniverse.open({
            residencies: RESIDENCIES,
            environment: simulation.environment("universe"),
            lifetimes: new Map([[relayPackage.id, TUNNEL_TOKEN_LIFETIME_MILLISECONDS]]),
        });
        opening.use(universe);

        // create the acme account in the residency as its owner
        const owner = await universe.signIn("owner@example.test");
        const client = owner.client(accountPackage);
        const created = await client.account.create({
            scope: owner.id,
            requestId: RequestId.create(simulation.environment("universe")),
            handle: HANDLE,
            name: "Acme",
            kind: "shared",
            residencyId: residency,
        });

        // enroll the computer, and place the work space on it
        const machine = await universe.enroll(owner, created.id, COMPUTER_ORIGIN);
        const space = await client.space.create({
            accountId: created.id,
            requestId: RequestId.create(simulation.environment("universe")),
            name: SPACE,
            machine: machine.id,
        });
        const computer = new TestTunnelMachine({
            simulation,
            universe,
            owner,
            machine,
            accountId: created.id,
            spaceId: space.id,
        });
        opening.move();

        return computer;
    }

    /** The clock the universe, the relays and the computer run on. */
    get clock(): SimulatedClock {
        return this.simulation.clock;
    }

    /** The name of the notes app of the computer's space. */
    get notes(): string {
        return `notes--${SPACE}--${HANDLE}.destack.space`;
    }

    /** Read the name of the computer itself. */
    async computer(): Promise<string> {
        const { name } = await this.accounts(this.machine).machine.get({
            scope: this.accountId,
            id: this.machine.id,
        });

        return `${name}--${HANDLE}.destack.computer`;
    }

    /** Run the simulation until a condition holds, within a relay token's lifetime of its time. */
    async until(condition: () => Promise<boolean> | boolean): Promise<void> {
        await this.simulation.run({
            until: condition,
            to: this.simulation.clock.now() + TUNNEL_TOKEN_LIFETIME_MILLISECONDS,
        });
    }

    /** Open a residency's cloud machine once, which hosts its relays and serves its spaces. */
    async host(residency: ResidencyCode): Promise<TestMachine> {
        const opened =
            this.#hosts.get(residency) ??
            (await this.simulation.drive(this.universe.machine(residency)));
        this.#hosts.set(residency, opened);

        return opened;
    }

    /** Start a relay installation in a residency's home, Europe by default. */
    async relay(
        options: {
            readonly residency?: ResidencyCode;
            readonly hostname?: string;
            readonly installation?: Subject;
        } = {},
    ): Promise<TestRelay> {
        // serve the relay as its residency's cloud machine hosts it, on a node of its own
        const { residency = "eu", hostname = `${residency}.relay.test`, ...rest } = options;
        const relay = await TestRelay.open({
            ...rest,
            hostname,
            universe: this.universe,
            host: await this.host(residency),
            environment: this.simulation.environment(`relay-${this.#relays}`),
        });
        this.#relays += 1;
        this.#started.use(relay);

        // wait until its replicas reached the universe
        await this.until(() => relay.isSettled());

        return relay;
    }

    /** Stop a relay before the scenario ends. */
    async stop(relay: TestRelay): Promise<void> {
        await relay[Symbol.asyncDispose]();
    }

    /** Wait until the relays' replicas reached the universe as of now. */
    async settle(...relays: readonly TestRelay[]): Promise<void> {
        await this.until(async () => {
            const settled = await Promise.all(relays.map((relay) => relay.isSettled()));

            return settled.every(Boolean);
        });
    }

    /** Read the computer's endpoint as the universe stores it, absent before it publishes. */
    async endpoint(): Promise<Endpoint | undefined> {
        return Snapshot.live(this.universe.storage.database)
            .read(endpoint.table)
            .findFirst({ where: { id: this.machine.id } });
    }

    /** Open a tunnel from the computer to a relay, which publishes its endpoint at each renewal. */
    async tunnel(
        relay: TestRelay,
        options: Partial<TunnelClientOptions> = {},
    ): Promise<TunnelClient> {
        // record each renewal's name, unless the scenario renews itself
        const { renewed = (renewal: Renewal) => void this.names.push(renewal.name), ...rest } =
            options;
        const first = Promise.withResolvers<void>();
        const renew = (renewal: Renewal) => {
            Promise.resolve(renewed(renewal)).then(
                () => first.resolve(),
                (error: unknown) =>
                    TelemetryScope.of(
                        import.meta.destack.package,
                        this.environment.telemetry,
                    ).captureException(error),
            );
        };

        // dial with the computer's relay tokens, recording and greeting each request
        const client = TunnelClient.open({
            url: relay.url,
            machineId: this.machine.id,
            token: () => this.machine.token(relayPackage.id),
            fetch: async (request) => {
                // record the request, and greet it with its method, path and body
                const url = new URL(request.url);
                this.received.push(`${request.method} ${url.host}${url.pathname}`);
                this.heads.push([...request.headers]);
                const body = request.body === null ? "" : ` ${await request.text()}`;

                return new Response(`hello ${request.method} ${url.pathname}${body}`);
            },
            renewed: renew,
            heartbeat: QUICK.heartbeat,
            retry: QUICK.retry,
            environment: this.simulation.environment("computer"),
            ...rest,
        });
        this.#started.defer(() => client.close());

        // return once the tunnel opened, the computer handled its first renewal, the relay follows it and published the tunnel
        await this.simulation.drive(Promise.all([client.opened(), first.promise]));
        await this.until(async () => {
            const published = await this.endpoint();

            return (await relay.isSettled()) && published?.relay?.url === new URL(relay.url).origin;
        });

        return client;
    }

    /** Serve the spaces of a residency's cloud machine at an origin, answering with a label. */
    async cloud(residency: ResidencyCode, label: string, origin: string): Promise<void> {
        // listen at the origin, answering the label and the Forwarded header
        const host = await this.host(residency);
        const listener = this.simulation.network.server(`cloud-${label}`).http({
            hostname: new URL(origin).hostname,
            port: Number(new URL(origin).port || "443"),
            fetch: (request) => new Response(`${label} ${request.headers.get("forwarded")}`),
        });
        this.#started.defer(() => listener.stop(true));

        // publish the origin as the cloud machine
        await this.accounts(host).machine.publish({
            scope: Scope.universe.id,
            id: host.id,
            requestId: RequestId.create(this.environment),
            url: origin,
        });
    }

    /** Create a space of the account on the residency's cloud machine, as its owner. */
    async place(name: string): Promise<Identifier<"space">> {
        const created = await this.owner.client(accountPackage).space.create({
            accountId: this.accountId,
            requestId: RequestId.create(this.environment),
            name,
        });

        return created.id;
    }

    /** Request a path of a name through a relay, answering the body or the refusal. */
    async request(
        relay: TestRelay,
        name: string,
        path: string,
        request: RequestInit = {},
    ): Promise<RelayAnswer> {
        // ask the relay's listener for the name, driving the simulation until it answers
        const headers = new Headers(request.headers);
        headers.set("host", name);
        const asked = this.simulation.network
            .of("caller")
            .fetch(new Request(`${relay.origin}${path}`, { ...request, headers }));
        const response = await this.simulation.drive(asked);

        // answer the body, or the refusal's code, reason and message
        if (response.ok) {
            return this.simulation.drive(response.text());
        }
        const { code, reason, message } = await this.simulation.drive(Failure.read(response));

        return [code, reason, message];
    }

    /** List the tunnel events the relays emitted, with their attributes, oldest first. */
    tunnels(): (readonly [string, Readonly<Record<string, unknown>>])[] {
        return this.telemetry
            .records()
            .flatMap((record) =>
                record.scope === relayPackage.name && record.event?.startsWith("tunnel.") === true
                    ? [[record.event, record.attributes] as const]
                    : [],
            );
    }

    /** Call the account package as a machine, the computer by default. */
    accounts(caller: TestMachine = this.machine) {
        return createClient(
            accountPackage,
            {
                url: caller.universe.accounts,
                fetch: caller.fetch(accountPackage.id),
            },
            this.simulation.environment(caller.id),
        );
    }

    /** Refuse exceptions the scenario left untaken, then stop everything it started. */
    async [Symbol.asyncDispose](): Promise<void> {
        try {
            // refuse the exceptions before the closes add their own
            this.telemetry[Symbol.dispose]();
        } finally {
            // stop the tunnels, listeners and relays, then the universe
            await this.#started.disposeAsync();
            await this.universe[Symbol.asyncDispose]();
        }
    }
}

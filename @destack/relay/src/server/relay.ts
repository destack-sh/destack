import type { WorkloadIdentity } from "@destack/account/client";
import { Resolver } from "@destack/account/directory";
import { principal } from "@destack/access";
import { and, Change, eq, isNull, type DatabaseConnection, type Table } from "@destack/db";
import { zoneTable } from "@destack/directory";
import { DOMAINS, type Domains, MachineAddress, InstallationOrigin } from "@destack/host";
import { account, key, Machine, machine, zone } from "@destack/account/object";
import { ObjectServer, Subscriber } from "@destack/object/server";
import { schema, type Identifier } from "@destack/schema";
import { Bearer, type TokenVerifier } from "@destack/service/authentication";
import type {} from "@destack/package/import-meta";
import { type Controller, TimerAlarm } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { refusal, reportError } from "@destack/service/server";
import { space, SpaceCell } from "@destack/space/object";
import { type Session, type Transport, TunnelProtocol } from "../session/index.ts";
import { type Renewal, TUNNEL_PATH, Tunnel } from "./tunnel.ts";
import { relayService } from "../service/index.ts";

/** The relay's package, the audience of the tokens machines open their tunnels with. */
export const RELAY_PACKAGE = import.meta.destack.package;

/** What a relay routes by, and where. */
export interface RelayOptions {
    /** The relay's own origin, where machines open their tunnels. */
    readonly origin: string;
    /** The relay's database, keeping copies of the accounts, machines, machine keys and zones names resolve with. */
    readonly database: DatabaseConnection;
    /** The relay's placement, which follows the account service's copies and reads names' claims and cells' endpoints through its directory. */
    readonly identity: WorkloadIdentity;
    /** Verify the universe's tokens machines open tunnels with, for the relay's package. */
    readonly tokens: TokenVerifier;
    /** Report a failure the relay answers to no one. */
    readonly report: (error: unknown) => void;
    /** The domains names resolve under. */
    readonly domains?: Domains;
    /** Reach regions at their published endpoints. */
    readonly fetch?: (request: Request) => Promise<Response>;
}

/** A machine admitted to open or renew a tunnel. */
export interface Admission {
    /** The machine. */
    readonly machineId: Identifier<"machine">;
    /** When its token lapses, in UTC epoch milliseconds. */
    readonly lapsesAt: number;
}

/** The edge of Destack's names: requests for a space's or a machine's name go to the machine through its tunnel, or to the region serving it, with the tunnels kept in memory. */
export class Relay {
    /** The copies of the rows names resolve with, followed from the account service as the relay's placement. */
    readonly objects: ObjectServer;
    /** The relay's database with the copies. */
    readonly #database: DatabaseConnection;
    /** The resolver of handles from the copies and of names through the directory. */
    readonly #resolver: Resolver;
    /** The verifier of machines' tokens. */
    readonly #tokens: TokenVerifier;
    /** The relay's own origin. */
    readonly #origin: URL;
    /** Report a failure the relay answers to no one. */
    readonly #report: (error: unknown) => void;
    /** The tunnel of each connected machine. */
    readonly #tunnels = new Map<string, Tunnel>();
    /** The domains names resolve under. */
    readonly #domains: Domains;
    /** Call regions. */
    readonly #fetch: (request: Request) => Promise<Response>;

    /** Route as the options describe. */
    constructor(options: RelayOptions) {
        // follow the relay's placement in the account service
        const { database, identity } = options;
        this.objects = new ObjectServer({
            objects: {},
            policies: [account, machine, key, zone],
            database,
            origin: { package: RELAY_PACKAGE, service: relayService.name },
            subscriber: Subscriber.of(identity.publisher(), () =>
                this.objects.source.workloadSubscriptions(identity.placementId),
            ),
        });

        // resolve handles from the copies and names through the directory
        this.#database = database;
        this.#resolver = new Resolver(identity.directory(), (handle) =>
            Resolver.account(database, handle),
        );

        // keep the token verifier and the calls to tunnels and regions
        this.#tokens = options.tokens;
        this.#origin = new URL(options.origin);
        this.#report = options.report;
        this.#domains = options.domains ?? DOMAINS;
        this.#fetch = options.fetch ?? globalThis.fetch;
    }

    /** The URL machines open their tunnels at. */
    get url(): string {
        return new URL(TUNNEL_PATH, this.#origin).href;
    }

    /** Admit a machine opening its tunnel at the relay's own origin, or answer a request for a name. */
    async route(request: Request): Promise<Admission | Response> {
        // forward requests for names
        const url = new URL(request.url);
        const isTunnel = url.host === this.#origin.host && url.pathname === TUNNEL_PATH;
        if (!isTunnel) {
            return this.fetch(request);
        }

        // admit the machine by its offered token, answering a refusal's status alone
        try {
            return await this.open(request);
        } catch (error) {
            const reported = reportError(error);

            return new Response(null, { status: reported.status });
        }
    }

    /** Run a session over a machine's opened WebSocket as the newest of its tunnel. */
    attach(admission: Admission, transport: Transport): Session {
        // find or keep the machine's tunnel, renewing only the machine's own tokens
        const { machineId, lapsesAt } = admission;
        const tunnel =
            this.#tunnels.get(machineId) ??
            new Tunnel(machineId, {
                verify: (request) => this.#renew(machineId, request),
                alarm: new TimerAlarm(() => void tunnel.lapse().catch(this.#report)),
                report: this.#report,
            });
        this.#tunnels.set(machineId, tunnel);

        return tunnel.attach(transport, lapsesAt);
    }

    /** Forget a machine's closed WebSocket, and the machine's tunnel once none of its WebSockets remain, reporting its failure. */
    detach(machineId: Identifier<"machine">, session: Session): void {
        this.#detach(machineId, session).catch(this.#report);
    }

    /** Tell a connected machine the name the relay routes to it now, as its tunnel's renewals answer it. */
    async rename(machineId: Identifier<"machine">): Promise<void> {
        // leave a machine without a tunnel here or a name
        const tunnel = this.#tunnels.get(machineId);
        const name = tunnel === undefined ? undefined : await this.name(machineId);
        if (tunnel === undefined || name === undefined) {
            return;
        }

        // send the name to the relay's tunnel path at the machine
        const response = await tunnel.fetch(
            new Request(this.url, { method: "PUT", body: JSON.stringify({ name }) }),
        );
        if (!response.ok) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `machine ${machineId} refused its name: ${response.status}`,
            });
        }
    }

    /** Verify the token of a machine opening or renewing its tunnel, with a standing key. */
    async admit(token: string, now = Date.now()): Promise<Admission> {
        // require a machine's token
        const caller = await this.#tokens.verify(token, undefined, now);
        const subject = caller.claims.subject;
        if (!principal.machine.is(subject)) {
            throw new ServiceError("FORBIDDEN", { message: "only machines open tunnels" });
        }

        // require its key to stand
        await Machine.requireAuthenticating(this.#database, caller, now);

        return {
            machineId: schema.identifier("machine").parse(subject.id),
            lapsesAt: caller.lapsesAt,
        };
    }

    /** Read the name the relay routes to a standing machine below the machine domain, as its account's copies name it now, absent for no standing machine. */
    async name(machineId: Identifier<"machine">): Promise<string | undefined> {
        const [named] = await this.#database
            .select({ machine: machine.table.name, handle: account.table.handle })
            .from(machine.table)
            .innerJoin(account.table, eq(account.table.id, machine.table.scope))
            .where(and(eq(machine.table.id, machineId), isNull(machine.table.revokedAt)));

        return named === undefined
            ? undefined
            : MachineAddress.format(named, this.#domains.machine);
    }

    /** List the standing machines of an account. */
    async machines(accountId: Identifier<"account">): Promise<readonly Identifier<"machine">[]> {
        const rows = await this.#database
            .select({ id: machine.table.id })
            .from(machine.table)
            .where(and(eq(machine.table.scope, accountId), isNull(machine.table.revokedAt)));

        return rows.map((row) => row.id);
    }

    /** Verify the token a machine opening its tunnel offers as a WebSocket subprotocol. */
    async open(request: Request, now = Date.now()): Promise<Admission> {
        const token = TunnelProtocol.token(request.headers.get("sec-websocket-protocol"));
        if (token === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer subprotocol" });
        }

        return this.admit(token, now);
    }

    /** Forward a request for a name to its cell, answering a failure with its status and message. */
    async fetch(request: Request): Promise<Response> {
        try {
            return await this.#forward(request);
        } catch (error) {
            return refusal(error);
        }
    }

    /** Forget a closed WebSocket's session, and its machine's tunnel without connections. */
    async #detach(machineId: Identifier<"machine">, session: Session): Promise<void> {
        // find the machine's tunnel
        const tunnel = this.#tunnels.get(machineId);
        if (tunnel === undefined) {
            return;
        }

        // forget the session, and the tunnel without connections
        await tunnel.detach(session);
        if (tunnel.isEmpty) {
            this.#tunnels.delete(machineId);
        }
    }

    /** Admit a renewal of a connection of a machine's own tunnel, answering when its new token lapses and the name the relay routes to it. */
    async #renew(machineId: Identifier<"machine">, request: Request): Promise<Renewal> {
        // admit only the machine's own token, renewed in the bearer header
        const admission = await this.admit(Bearer.require(request.headers));
        if (admission.machineId !== machineId) {
            throw new ServiceError("FORBIDDEN", {
                message: "machines only renew their own tunnels",
            });
        }

        // answer the name the relay routes to the machine
        const name = await this.name(machineId);
        if (name === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `the relay names no machine ${machineId} yet`,
            });
        }

        return { lapsesAt: admission.lapsesAt, name };
    }

    /** Forward a request for a name: to a machine through its tunnel, or to a region at its endpoint. */
    async #forward(request: Request): Promise<Response> {
        // find the name's cell
        const name = new URL(request.url).hostname;
        const cell = await this.#cell(name);

        // refuse a name that resolves to nothing
        if (cell === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${name} resolves to nothing` });
        }
        // reach a machine through its tunnel
        else if ("machineId" in cell) {
            const tunnel = this.#tunnels.get(cell.machineId);
            if (tunnel === undefined) {
                throw new ServiceError("SERVICE_UNAVAILABLE", {
                    message: `machine ${cell.machineId} is not connected`,
                });
            }

            return tunnel.fetch(request);
        }
        // reach a region at its published endpoint, naming the machine asked for
        else {
            return this.#fetch(await this.#regional(request, cell.regionId));
        }
    }

    /** Find the machine or region serving a name. */
    async #cell(name: string): Promise<SpaceCell | undefined> {
        const origin = InstallationOrigin.parse(name, this.#domains.space);
        const computer = MachineAddress.parse(name, this.#domains.machine);

        // find the cell of a space's copied zone, which also serves its branches
        if (origin !== undefined) {
            const named = await this.#resolver.find(space, `${origin.space}.${origin.handle}`);
            const [placed] =
                named === undefined
                    ? []
                    : await this.#database
                          .select({ cell: zoneTable.cell })
                          .from(zoneTable)
                          .where(eq(zoneTable.id, named.id));

            return placed === undefined ? undefined : SpaceCell.parse(placed.cell);
        }
        // find a standing machine by its name within its account
        else if (computer !== undefined) {
            const accountId = await this.#resolver.account(computer.handle);
            const machineId =
                accountId === undefined
                    ? undefined
                    : await Machine.find(this.#database, accountId, computer.machine);

            return machineId === undefined ? undefined : { machineId };
        }
        // resolve nothing else
        else {
            return undefined;
        }
    }

    /** Address a request to a region's published endpoint, keeping the name asked for (RFC 7239). */
    async #regional(request: Request, regionId: string): Promise<Request> {
        // read the endpoint
        const published = await this.#resolver.directory.cell(regionId);
        if (published === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `region ${regionId} published no endpoint`,
            });
        }

        // keep the path, query and streamed body under the endpoint's origin
        const url = new URL(request.url);
        const target = new URL(`${url.pathname}${url.search}`, published.endpoint);
        const forwarded = new Request(target.href, request);
        forwarded.headers.set(
            "forwarded",
            `machine="${url.host}";proto=${url.protocol.slice(0, -1)}`,
        );

        return forwarded;
    }
}

/** Tell each connected machine its name once its own or its account's copy changes, as a rename or a new handle does. */
export class NameController implements Controller {
    /** The controller's name in reports. */
    readonly name = "name";
    /** The copied machines and accounts names derive from. */
    readonly watches: readonly Table[] = [machine.table, account.table];
    /** The relay keeping the tunnels. */
    readonly #relay: Relay;

    /** Tell the machines connected to a relay their names. */
    constructor(relay: Relay) {
        this.#relay = relay;
    }

    /** List the machine whose copy changed, or the machines of an account whose copy changed. */
    async keys(change: Change): Promise<readonly string[]> {
        // select a changed machine
        if (Change.of(change, machine.table)) {
            return [Change.image(change).id];
        }
        // select the machines of a changed account
        else if (Change.of(change, account.table)) {
            return this.#relay.machines(Change.image(change).id);
        }
        // select nothing for other tables
        else {
            return [];
        }
    }

    /** List no machine at start, since each tunnel learns its name as it opens. */
    async list(): Promise<readonly string[]> {
        return [];
    }

    /** Tell a connected machine its name. */
    async reconcile(machineId: string): Promise<undefined> {
        await this.#relay.rename(schema.identifier("machine").parse(machineId));

        return undefined;
    }
}

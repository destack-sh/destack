import type { WorkloadIdentity } from "@destack/account/client";
import { Resolver } from "@destack/account/directory";
import { principal } from "@destack/access";
import { and, Change, eq, isNull, type DatabaseConnection, type Table } from "@destack/db";
import { zoneTable } from "@destack/directory";
import { DOMAINS, type Domains, HostAddress, InstallationOrigin } from "@destack/host";
import { account, Host, HostKey, host, hostKey, zone } from "@destack/account/object";
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

/** The relay's package, the audience of the tokens hosts open their tunnels with. */
export const RELAY_PACKAGE = import.meta.destack.package;

/** What a relay routes by, and where. */
export interface RelayOptions {
    /** The relay's own origin, where hosts open their tunnels. */
    readonly origin: string;
    /** The relay's database, keeping copies of the accounts, hosts, host keys and zones names resolve with. */
    readonly database: DatabaseConnection;
    /** The relay's placement, which follows the account service's copies and reads names' claims and cells' endpoints through its directory. */
    readonly identity: WorkloadIdentity;
    /** Verify the universe's tokens hosts open tunnels with, for the relay's package. */
    readonly tokens: TokenVerifier;
    /** Report a failure the relay answers to no one. */
    readonly report: (error: unknown) => void;
    /** The domains names resolve under. */
    readonly domains?: Domains;
    /** Reach regions at their published endpoints. */
    readonly fetch?: (request: Request) => Promise<Response>;
}

/** A host admitted to open or renew a tunnel. */
export interface Admission {
    /** The host. */
    readonly hostId: Identifier<"host">;
    /** When its token lapses, in UTC epoch milliseconds. */
    readonly lapsesAt: number;
}

/** The edge of Destack's names: requests for a space's or a host's name go to the host through its tunnel, or to the region serving it, with the tunnels kept in memory. */
export class Relay {
    /** The copies of the rows names resolve with, followed from the account service as the relay's placement. */
    readonly objects: ObjectServer;
    /** The relay's database with the copies. */
    readonly #database: DatabaseConnection;
    /** The resolver of handles from the copies and of names through the directory. */
    readonly #resolver: Resolver;
    /** The verifier of hosts' tokens. */
    readonly #tokens: TokenVerifier;
    /** The relay's own origin. */
    readonly #origin: URL;
    /** Report a failure the relay answers to no one. */
    readonly #report: (error: unknown) => void;
    /** The tunnel of each connected host. */
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
            policies: [account, host, hostKey, zone],
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

    /** The URL hosts open their tunnels at. */
    get url(): string {
        return new URL(TUNNEL_PATH, this.#origin).href;
    }

    /** Admit a host opening its tunnel at the relay's own origin, or answer a request for a name. */
    async route(request: Request): Promise<Admission | Response> {
        // forward requests for names
        const url = new URL(request.url);
        const isTunnel = url.host === this.#origin.host && url.pathname === TUNNEL_PATH;
        if (!isTunnel) {
            return this.fetch(request);
        }

        // admit the host by its offered token, answering a refusal's status alone
        try {
            return await this.open(request);
        } catch (error) {
            const reported = reportError(error);

            return new Response(null, { status: reported.status });
        }
    }

    /** Run a session over a host's opened WebSocket as the newest of its tunnel. */
    attach(admission: Admission, transport: Transport): Session {
        // find or keep the host's tunnel, renewing only the host's own tokens
        const { hostId, lapsesAt } = admission;
        const tunnel =
            this.#tunnels.get(hostId) ??
            new Tunnel(hostId, {
                verify: (request) => this.#renew(hostId, request),
                alarm: new TimerAlarm(() => void tunnel.lapse().catch(this.#report)),
                report: this.#report,
            });
        this.#tunnels.set(hostId, tunnel);

        return tunnel.attach(transport, lapsesAt);
    }

    /** Forget a host's closed WebSocket, and the host's tunnel once none of its WebSockets remain, reporting its failure. */
    detach(hostId: Identifier<"host">, session: Session): void {
        this.#detach(hostId, session).catch(this.#report);
    }

    /** Tell a connected host the name the relay routes to it now, as its tunnel's renewals answer it. */
    async rename(hostId: Identifier<"host">): Promise<void> {
        // leave a host without a tunnel here or a name
        const tunnel = this.#tunnels.get(hostId);
        const name = tunnel === undefined ? undefined : await this.name(hostId);
        if (tunnel === undefined || name === undefined) {
            return;
        }

        // send the name to the relay's tunnel path at the host
        const response = await tunnel.fetch(
            new Request(this.url, { method: "PUT", body: JSON.stringify({ name }) }),
        );
        if (!response.ok) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `host ${hostId} refused its name: ${response.status}`,
            });
        }
    }

    /** Verify the token of a host opening or renewing its tunnel, with a standing key. */
    async admit(token: string, now = Date.now()): Promise<Admission> {
        // require a host's token
        const caller = await this.#tokens.verify(token, undefined, now);
        const subject = caller.claims.subject;
        if (!principal.host.is(subject)) {
            throw new ServiceError("FORBIDDEN", { message: "only hosts open tunnels" });
        }

        // require its key to stand
        await HostKey.requireAuthenticating(this.#database, caller, now);

        return { hostId: schema.identifier("host").parse(subject.id), lapsesAt: caller.lapsesAt };
    }

    /** Read the name the relay routes to a standing host below the host domain, as its account's copies name it now, absent for no standing host. */
    async name(hostId: Identifier<"host">): Promise<string | undefined> {
        const [named] = await this.#database
            .select({ host: host.table.name, handle: account.table.handle })
            .from(host.table)
            .innerJoin(account.table, eq(account.table.id, host.table.scope))
            .where(and(eq(host.table.id, hostId), isNull(host.table.revokedAt)));

        return named === undefined ? undefined : HostAddress.format(named, this.#domains.host);
    }

    /** List the standing hosts of an account. */
    async hosts(accountId: Identifier<"account">): Promise<readonly Identifier<"host">[]> {
        const rows = await this.#database
            .select({ id: host.table.id })
            .from(host.table)
            .where(and(eq(host.table.scope, accountId), isNull(host.table.revokedAt)));

        return rows.map((row) => row.id);
    }

    /** Verify the token a host opening its tunnel offers as a WebSocket subprotocol. */
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

    /** Forget a closed WebSocket's session, and its host's tunnel without connections. */
    async #detach(hostId: Identifier<"host">, session: Session): Promise<void> {
        // find the host's tunnel
        const tunnel = this.#tunnels.get(hostId);
        if (tunnel === undefined) {
            return;
        }

        // forget the session, and the tunnel without connections
        await tunnel.detach(session);
        if (tunnel.isEmpty) {
            this.#tunnels.delete(hostId);
        }
    }

    /** Admit a renewal of a connection of a host's own tunnel, answering when its new token lapses and the name the relay routes to it. */
    async #renew(hostId: Identifier<"host">, request: Request): Promise<Renewal> {
        // admit only the host's own token, renewed in the bearer header
        const admission = await this.admit(Bearer.require(request.headers));
        if (admission.hostId !== hostId) {
            throw new ServiceError("FORBIDDEN", { message: "hosts only renew their own tunnels" });
        }

        // answer the name the relay routes to the host
        const name = await this.name(hostId);
        if (name === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `the relay names no host ${hostId} yet`,
            });
        }

        return { lapsesAt: admission.lapsesAt, name };
    }

    /** Forward a request for a name: to a host through its tunnel, or to a region at its endpoint. */
    async #forward(request: Request): Promise<Response> {
        // find the name's cell
        const name = new URL(request.url).hostname;
        const cell = await this.#cell(name);

        // refuse a name that resolves to nothing
        if (cell === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${name} resolves to nothing` });
        }
        // reach a host through its tunnel
        else if ("hostId" in cell) {
            const tunnel = this.#tunnels.get(cell.hostId);
            if (tunnel === undefined) {
                throw new ServiceError("SERVICE_UNAVAILABLE", {
                    message: `host ${cell.hostId} is not connected`,
                });
            }

            return tunnel.fetch(request);
        }
        // reach a region at its published endpoint, naming the host asked for
        else {
            return this.#fetch(await this.#regional(request, cell.regionId));
        }
    }

    /** Find the host or region serving a name. */
    async #cell(name: string): Promise<SpaceCell | undefined> {
        const origin = InstallationOrigin.parse(name, this.#domains.space);
        const computer = HostAddress.parse(name, this.#domains.host);

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
        // find a standing host by its name within its account
        else if (computer !== undefined) {
            const accountId = await this.#resolver.account(computer.handle);
            const hostId =
                accountId === undefined
                    ? undefined
                    : await Host.find(this.#database, accountId, computer.host);

            return hostId === undefined ? undefined : { hostId };
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
        forwarded.headers.set("forwarded", `host="${url.host}";proto=${url.protocol.slice(0, -1)}`);

        return forwarded;
    }
}

/** Tell each connected host its name once its own or its account's copy changes, as a rename or a new handle does. */
export class NameController implements Controller {
    /** The controller's name in reports. */
    readonly name = "name";
    /** The copied hosts and accounts names derive from. */
    readonly watches: readonly Table[] = [host.table, account.table];
    /** The relay keeping the tunnels. */
    readonly #relay: Relay;

    /** Tell the hosts connected to a relay their names. */
    constructor(relay: Relay) {
        this.#relay = relay;
    }

    /** List the host whose copy changed, or the hosts of an account whose copy changed. */
    async keys(change: Change): Promise<readonly string[]> {
        // select a changed host
        if (Change.of(change, host.table)) {
            return [Change.image(change).id];
        }
        // select the hosts of a changed account
        else if (Change.of(change, account.table)) {
            return this.#relay.hosts(Change.image(change).id);
        }
        // select nothing for other tables
        else {
            return [];
        }
    }

    /** List no host at start, since each tunnel learns its name as it opens. */
    async list(): Promise<readonly string[]> {
        return [];
    }

    /** Tell a connected host its name. */
    async reconcile(key: string): Promise<undefined> {
        await this.#relay.rename(schema.identifier("host").parse(key));

        return undefined;
    }
}

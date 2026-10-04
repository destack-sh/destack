import type { WorkloadIdentity } from "@destack/account/client";
import { Resolver } from "@destack/account/directory";
import { principal } from "@destack/access";
import { and, eq, isNull, type DatabaseConnection } from "@destack/db";
import { zoneTable } from "@destack/directory";
import { DOMAINS, type Domains, HostAddress, InstallationOrigin } from "@destack/host";
import { account, Host, HostKey, host, hostKey, zone } from "@destack/account/object";
import { ObjectServer, Subscriber } from "@destack/object/server";
import { schema, type Identifier } from "@destack/schema";
import type { TokenVerifier } from "@destack/service/authentication";
import type {} from "@destack/package/import-meta";
import { ServiceError } from "@destack/service/error";
import { refusal } from "@destack/service/server";
import { space, SpaceCell } from "@destack/space/object";
import { TunnelProtocol } from "../session/index.ts";
import type { Tunnel } from "./tunnel.ts";

/** The relay's package, the audience of the tokens hosts open their tunnels with. */
export const RELAY_PACKAGE = import.meta.destack.package;

/** What a relay routes by, and where. */
export interface RelayOptions {
    /** The relay's database, keeping copies of the accounts, hosts, host keys and zones names resolve with. */
    readonly database: DatabaseConnection;
    /** The relay's placement, which follows the account service's copies and reads names' claims and cells' endpoints through its directory. */
    readonly identity: WorkloadIdentity;
    /** Verify the universe's tokens hosts open tunnels with, for the relay's package. */
    readonly tokens: TokenVerifier;
    /** Find a host's tunnel, absent while the host keeps none open here. */
    readonly tunnel: (hostId: Identifier<"host">) => Pick<Tunnel, "fetch"> | undefined;
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

/** The edge of Destack's names: requests for a space's or a host's name go to the host through its tunnel, or to the region serving it. */
export class Relay {
    /** The copies of the rows names resolve with, followed from the account service as the relay's placement. */
    readonly objects: ObjectServer;
    /** The relay's database with the copies. */
    readonly #database: DatabaseConnection;
    /** The resolver of handles from the copies and of names through the directory. */
    readonly #resolver: Resolver;
    /** The verifier of hosts' tokens. */
    readonly #tokens: TokenVerifier;
    /** Find a host's tunnel. */
    readonly #tunnel: RelayOptions["tunnel"];
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
            origin: { package: RELAY_PACKAGE, service: "relay" },
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
        this.#tunnel = options.tunnel;
        this.#domains = options.domains ?? DOMAINS;
        this.#fetch = options.fetch ?? globalThis.fetch;
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
            const tunnel = this.#tunnel(cell.hostId);
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

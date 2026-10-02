import type { Resolver } from "@destack/account/directory";
import { principal } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import { DOMAINS, type Domains, HostAddress, InstallationOrigin } from "@destack/host";
import { Host, HostKey } from "@destack/account/object";
import type { Identifier } from "@destack/schema";
import type { TokenVerifier } from "@destack/service/authentication";
import type {} from "@destack/package/import-meta";
import { ServiceError } from "@destack/service/error";
import { refusal } from "@destack/service/server";
import { space, SpaceCell } from "@destack/space/object";
import type { Tunnel } from "./tunnel.ts";

/** The relay's package, the audience of the tokens hosts open their tunnels with. */
export const RELAY_PACKAGE = import.meta.destack.package;

/** What a relay routes by, and where. */
export interface RelayOptions {
    /** The global database: accounts, hosts and their keys. */
    readonly database: DatabaseConnection;
    /** The resolver for names and the directory of cells. */
    readonly resolver: Resolver;
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
    /** The global database. */
    readonly #database: DatabaseConnection;
    /** The resolver names resolve through. */
    readonly #resolver: Resolver;
    /** The verifier of hosts' tokens. */
    readonly #tokens: TokenVerifier;
    /** Find a host's tunnel. */
    readonly #tunnel: RelayOptions["tunnel"];
    /** The domains names resolve under. */
    readonly #domains: Domains;
    /** Reach regions. */
    readonly #fetch: (request: Request) => Promise<Response>;

    /** Route as the options describe. */
    constructor(options: RelayOptions) {
        // keep the global state, the verifier and the reach to tunnels and regions
        this.#database = options.database;
        this.#resolver = options.resolver;
        this.#tokens = options.tokens;
        this.#tunnel = options.tunnel;
        this.#domains = options.domains ?? DOMAINS;
        this.#fetch = options.fetch ?? globalThis.fetch;
    }

    /** Verify the token of a host opening or renewing its tunnel, with a standing key. */
    async admit(request: Request, now = Date.now()): Promise<Admission> {
        // require a host's token
        const caller = await this.#tokens.authenticate(request, undefined, now);
        const subject = caller.claims.subject;
        if (!principal.host.is(subject)) {
            throw new ServiceError("FORBIDDEN", { message: "only hosts open tunnels" });
        }

        // require its key to stand
        await HostKey.requireAuthenticating(this.#database, caller, now);

        return { hostId: subject.id as Identifier<"host">, lapsesAt: caller.lapsesAt };
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
        const host = HostAddress.parse(name, this.#domains.host);

        // find the cell of a space's zone, which also serves its branches
        if (origin !== undefined) {
            const named = await this.#resolver.find(space, `${origin.space}.${origin.handle}`);
            const zone =
                named === undefined ? undefined : await this.#resolver.directory.locate(named.id);

            return zone === undefined ? undefined : SpaceCell.parse(zone.cell);
        }
        // find a standing host by its name within its account
        else if (host !== undefined) {
            const accountId = await this.#resolver.account(host.handle);
            const hostId =
                accountId === undefined
                    ? undefined
                    : await Host.find(this.#database, accountId, host.host);

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

        // keep the path and query under the endpoint's origin
        const url = new URL(request.url);
        const target = new URL(`${url.pathname}${url.search}`, published.endpoint);
        const headers = new Headers(request.headers);
        headers.set("forwarded", `host="${url.host}";proto=${url.protocol.slice(0, -1)}`);

        // stream the body on
        const initialize = { method: request.method, headers, body: request.body, duplex: "half" };

        return new Request(target.href, initialize as RequestInit);
    }
}

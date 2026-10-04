import { account, host } from "@destack/account/object";
import { Change, type Table } from "@destack/db";
import { type Identifier, schema } from "@destack/schema";
import { Bearer } from "@destack/service/authentication";
import { type Controller, ControlLoop, TimerAlarm } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { reportError } from "@destack/service/server";
import type { Session, Transport } from "../session/index.ts";
import { type Admission, Relay, type RelayOptions } from "./relay.ts";
import { type Renewal, TUNNEL_PATH, Tunnel } from "./tunnel.ts";

/** What a relay serves with on any runtime. */
export interface RelayServerOptions extends Omit<RelayOptions, "tunnel"> {
    /** The relay's own origin, where hosts open their tunnels. */
    readonly origin: string;
    /** Report a failure the relay answers to no one. */
    readonly report: (error: unknown) => void;
}

/** A relay in one process: its names and its hosts' tunnels, the tunnels kept in memory, served by a runtime's WebSockets. */
export class RelayServer {
    /** The relay routing names. */
    readonly relay: Relay;
    /** The relay's own origin. */
    readonly #origin: URL;
    /** Report a failure the relay answers to no one. */
    readonly #report: (error: unknown) => void;
    /** The tunnel of each connected host. */
    readonly #tunnels = new Map<string, Tunnel>();
    /** Stops following the copies. */
    readonly #stopping = new AbortController();
    /** The control loop following the copies. */
    readonly #following: Promise<void>;

    /** Serve a relay as the options describe. */
    private constructor(options: RelayServerOptions) {
        // route names to the tunnels kept here
        this.#origin = new URL(options.origin);
        this.#report = options.report;
        this.relay = new Relay({ ...options, tunnel: (hostId) => this.#tunnels.get(hostId) });

        // follow the copies names resolve with, telling connected hosts their changed names
        const loop = new ControlLoop(
            options.database,
            [...this.relay.objects.controllers(), new NameController(this)],
            { report: (_controller, _key, error) => options.report(error) },
        );
        this.#following = loop.run(this.#stopping.signal).catch(options.report);
    }

    /** Start following the copies, serving once a runtime hands requests and sockets to the relay. */
    static start(options: RelayServerOptions): RelayServer {
        return new RelayServer(options);
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
            return this.relay.fetch(request);
        }

        // admit the host by its offered token, answering a refusal's status alone
        try {
            return await this.relay.open(request);
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

    /** Stop following the copies. */
    async close(): Promise<void> {
        this.#stopping.abort();
        await this.#following;
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
        const admission = await this.relay.admit(Bearer.require(request.headers));
        if (admission.hostId !== hostId) {
            throw new ServiceError("FORBIDDEN", { message: "hosts only renew their own tunnels" });
        }

        // answer the name the relay routes to the host
        const name = await this.relay.name(hostId);
        if (name === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `the relay names no host ${hostId} yet`,
            });
        }

        return { lapsesAt: admission.lapsesAt, name };
    }

    /** Tell a connected host the name the relay routes to it now, as its tunnel's renewals answer it. */
    async rename(hostId: Identifier<"host">): Promise<void> {
        // leave a host without a tunnel here or a name
        const tunnel = this.#tunnels.get(hostId);
        const name = tunnel === undefined ? undefined : await this.relay.name(hostId);
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
}

/** Tell each connected host its name once its own or its account's copy changes, as a rename or a new handle does. */
class NameController implements Controller {
    /** The controller's name in reports. */
    readonly name = "name";
    /** The copied hosts and accounts names derive from. */
    readonly watches: readonly Table[] = [host.table, account.table];
    /** The relay server keeping the tunnels. */
    readonly #server: RelayServer;

    /** Tell the hosts connected to a relay server their names. */
    constructor(server: RelayServer) {
        this.#server = server;
    }

    /** List the host whose copy changed, or the hosts of an account whose copy changed. */
    async keys(change: Change): Promise<readonly string[]> {
        // select a changed host
        if (Change.of(change, host.table)) {
            return [Change.image(change).id];
        }
        // select the hosts of a changed account
        else if (Change.of(change, account.table)) {
            return this.#server.relay.hosts(Change.image(change).id);
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
        await this.#server.rename(schema.identifier("host").parse(key));

        return undefined;
    }
}

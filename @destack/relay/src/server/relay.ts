import {
    Directory,
    type Domains,
    InstallationOrigin,
    MachineAddress,
    SpaceAddress,
} from "@destack/account/address";
import {
    account,
    endpoint,
    Endpoint,
    key,
    machine,
    Placement,
    type ResidencyCode,
    space,
} from "@destack/account/object";
import type { Snapshot } from "@destack/db";
import type { Clock, Environment } from "@destack/environment";
import { Failure } from "@destack/error";
import { type Identifier, schema } from "@destack/schema";
import type { Fetch } from "@destack/object";
import { PackageMount, refusal, reportError } from "@destack/object";
import {
    FORWARDED_HEAD_HEADER,
    ForwardedRequest,
    PARTITION_HEADER,
} from "@destack/object/workload";
import { RelayError } from "../error/index.ts";
import relayPackage from "../package.ts";
import { TunnelProtocol } from "../session/index.ts";
import {
    HANDSHAKE_HEADER,
    type Handshake,
    STANDING_ERRORS,
    Tunnel,
    type TunnelCloseReason,
    type TunnelStanding,
    TUNNEL_PATH,
    TUNNELS_PATH,
    type TunnelCall,
} from "./tunnel.ts";
import { TunnelToken } from "./token.ts";

/** The parameters a relay sets in the Forwarded header of a request it forwards (RFC 7239). */
const FORWARDED = {
    /** The host the client asked for. */
    host: /host="?([^";,]+)"?/u,
    /** The protocol the client asked with. */
    proto: /proto=([a-z]+)/u,
} as const;

/** What a relay routes by, and where. */
export interface RelayOptions {
    /** The relay's own origin, where machines open their tunnels. */
    readonly origin: string;
    /** The universe's issuer, which machines' tokens carry. */
    readonly issuer: string;
    /** The residency the relay serves, whose accounts' machines alone tunnel to it. */
    readonly residency: ResidencyCode;
    /** The relay installation itself, which calls each machine's tunnel in its partition. */
    readonly itself: { readonly url: string; readonly fetch: Fetch };
    /** The replica of the universe's directory and identities. */
    readonly view: Snapshot;
    /** The domains names resolve under. */
    readonly domains: Domains;
    /** Call cloud machines at their published endpoints. */
    readonly fetch: Fetch;
    /** The clock tokens and endpoints lapse on, and the WebCrypto verifying the tokens. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "network" | "telemetry">;
    /** Wait until the directory's and the residents' replicas synced, which standing reads. */
    readonly synced: () => Promise<void>;
}

/** Where the relay forwards a name's requests: a machine's tunnel, or a published endpoint. */
export type Destination =
    | { readonly kind: "tunnel"; readonly machineId: Identifier<"machine"> }
    | { readonly kind: "endpoint"; readonly url: string };

/** The edge of Destack's names. */
export class Relay {
    /** The domains names resolve under. */
    readonly domains: Domains;

    /** The replica of the directory and the identities: placements, machines and keys. */
    readonly #view: Snapshot;
    /** Wait until the replicas standing reads synced. */
    readonly #synced: () => Promise<void>;

    /** The directory over the replica, which resolves names, handles and placements. */
    readonly #directory: Directory;

    /** The relay installation itself, which calls each machine's tunnel in its partition. */
    readonly #itself: RelayOptions["itself"];

    /** The clock tokens and endpoints lapse on. */
    readonly #clock: Clock;

    /** The universe's tunnel tokens, which machines open their tunnels with. */
    readonly #tokens: TunnelToken;

    /** The relay's own origin. */
    readonly #origin: URL;

    /** The residency the relay serves. */
    readonly #residency: ResidencyCode;

    /** Call cloud machines. */
    readonly #fetch: Fetch;

    /** The telemetry the relay's refused requests are captured on. */
    readonly #telemetry: Environment["telemetry"];

    /** Route as the options describe. */
    constructor(options: RelayOptions) {
        // resolve handles and names through the replicas
        this.#view = options.view;
        this.#synced = options.synced;
        this.#directory = new Directory(options.view);
        this.#clock = options.environment.clock;
        this.#telemetry = options.environment.telemetry;

        // set the token verifier and the calls to tunnels and cloud machines
        this.#tokens = new TunnelToken(options.issuer, this.#view, options.environment);
        this.domains = options.domains;
        this.#itself = options.itself;
        this.#origin = new URL(options.origin);
        this.#residency = options.residency;
        this.#fetch = options.fetch;
    }

    /** The URL machines open their tunnels at. */
    get url(): string {
        return new URL(TUNNEL_PATH, this.#origin).href;
    }

    /** Open a machine's tunnel, and relay every other request to its name's machine. */
    async fetch(request: Request): Promise<Response> {
        // forward requests for names
        const url = new URL(request.url);
        const isTunnel = url.host === this.#origin.host && url.pathname === TUNNEL_PATH;
        if (!isTunnel) {
            return this.#relay(request);
        }

        // accept the machine's handshake and open its tunnel, answering a refusal's status alone
        try {
            const handshake = await this.handshake(request);

            return await this.call(handshake.machineId, "open", {
                method: "POST",
                headers: { [HANDSHAKE_HEADER]: JSON.stringify(handshake) },
            });
        } catch (error) {
            const reported = reportError(error, this.#telemetry);

            return new Response(null, { status: reported.status });
        }
    }

    /** Accept a machine's opening handshake with the token it offers as a WebSocket subprotocol. */
    async handshake(request: Request): Promise<Handshake> {
        // require a machine's token
        const token = TunnelProtocol.token(request.headers.get("sec-websocket-protocol"));
        if (token === undefined) {
            throw RelayError.of("SUBPROTOCOL_INVALID", "invalid bearer subprotocol");
        }
        const { machineId, lapsesAt, kid } = await this.#tokens.verify(token, this.#clock.now());

        // require the machine to stand with a standing key
        const standing = await this.standing(machineId);
        if ("reason" in standing) {
            const [reason, refused] = STANDING_ERRORS[standing.reason];
            throw RelayError.of(reason, `machine ${machineId} ${refused}`);
        }

        // require its account to live in the relay's residency
        const resident = await this.#view
            .read(account.table)
            .findFirst({ where: { id: standing.account } });
        if (resident?.residencyId !== this.#residency) {
            throw RelayError.of(
                "RESIDENCY_FOREIGN",
                `machine ${machineId} tunnels through the relay of its account's residency`,
            );
        }

        return { machineId, lapsesAt, kid, name: standing.name, account: standing.account };
    }

    /** List the standing machines of an account. */
    async machines(accountId: Identifier<"account">): Promise<readonly Identifier<"machine">[]> {
        const rows = await this.#view
            .read(machine.table)
            .findMany({ where: { scope: accountId, revokedAt: { isNull: true } } });

        return rows.map((row) => row.id);
    }

    /** Report whether an endpoint leads to a tunnel at this relay. */
    isHere(published: Pick<Endpoint, "relay">): boolean {
        return published.relay !== null && new URL(published.relay.url).host === this.#origin.host;
    }

    /** List the machines whose endpoints lead to tunnels here. */
    async tunnelled(): Promise<readonly Identifier<"machine">[]> {
        const relayed = { relay: { isNotNull: true } } as const;
        const rows = await this.#view.read(endpoint.table).findMany({ where: relayed });

        return rows.filter((row) => this.isHere(row)).map((row) => row.id);
    }

    /** Report whether a machine's endpoint leads to a tunnel here. */
    async isTunnelled(machineId: Identifier<"machine">): Promise<boolean> {
        const published = await this.#view
            .read(endpoint.table)
            .findFirst({ where: { id: machineId } });

        return published !== undefined && this.isHere(published);
    }

    /** Read whether a machine stands with a standing key: its name, or why its tunnel ends. */
    async standing(machineId: Identifier<"machine">): Promise<TunnelStanding> {
        // wait for the synced replicas, then read the machine and its keys
        await this.#synced();
        const replicated = await this.#machine(machineId);
        if (replicated === undefined) {
            return { reason: "machine-unknown" };
        } else if (replicated.isRevoked) {
            return { reason: "machine-revoked" };
        } else if (!(await this.#isKeyed(machineId, this.#clock.now()))) {
            return { reason: "key-revoked" };
        }

        return { name: replicated.name, account: replicated.account };
    }

    /** Bring a machine's tunnel here in line with its replica: its name, or its close. */
    async reconcile(machineId: Identifier<"machine">): Promise<void> {
        // read whether the machine stands, and where its endpoint leads
        const standing = await this.standing(machineId);
        const published = await this.#view
            .read(endpoint.table)
            .findFirst({ where: { id: machineId } });

        // close the tunnel of a machine that no longer stands
        if ("reason" in standing) {
            await this.#post(machineId, "close", { reason: standing.reason });
        }
        // tell a tunnel still here its machine's name, which a renamed machine renews to learn
        else if (published !== undefined && this.isHere(published)) {
            await this.#post(machineId, "rename", { name: standing.name });
        }
        // close the tunnel here of a machine whose endpoint moved to another relay, or that unpublished it
        else {
            await this.#post(machineId, "close", {
                reason: published === undefined ? "closed" : "moved",
            });
        }
    }

    /** Post a body to a machine's tunnel, failing with its refusal. */
    async #post(
        machineId: Identifier<"machine">,
        call: Extract<TunnelCall, "close" | "rename">,
        body: { readonly reason: TunnelCloseReason } | { readonly name: string },
    ): Promise<void> {
        const response = await this.call(machineId, call, {
            method: "POST",
            body: JSON.stringify(body),
        });
        if (!response.ok) {
            throw Failure.error(await Failure.read(response));
        }
    }

    /** Call a machine's tunnel in its partition through the relay installation itself. */
    call(
        machineId: Identifier<"machine">,
        call: TunnelCall,
        options: RequestInit,
    ): Promise<Response> {
        // address the machine's tunnel partition
        const headers = new Headers(options.headers);
        headers.set(PARTITION_HEADER, Tunnel.partition(machineId));
        const path = `${PackageMount.path(relayPackage.id)}${TUNNELS_PATH}`;
        const url = `${this.#itself.url}${path}/${machineId}/${call}`;

        return this.#itself.fetch(new Request(url, { ...options, headers }));
    }

    /** Relay a request for a name to its machine. */
    async #relay(request: Request): Promise<Response> {
        try {
            const restored = this.#restore(request);

            return await (restored === undefined
                ? this.#forward(request, false)
                : this.#forward(restored, true));
        } catch (error) {
            return refusal(error, this.#telemetry);
        }
    }

    /** Restore a request another relay forwarded to this relay's origin (RFC 7239). */
    #restore(request: Request): Request | undefined {
        // take a request at the relay's origin with a Forwarded header
        const url = new URL(request.url);
        const forwarded = request.headers.get("forwarded");
        if (url.host !== this.#origin.host || forwarded === null) {
            return undefined;
        }

        // require the forwarded host and protocol
        const host = FORWARDED.host.exec(forwarded)?.[1];
        const proto = FORWARDED.proto.exec(forwarded)?.[1];
        if (host === undefined || proto === undefined) {
            throw RelayError.of(
                "FORWARDED_HOST_MISSING",
                "the forwarded request has no host or protocol",
            );
        }

        // ask for the name as the client did
        const restored = new Request(`${proto}://${host}${url.pathname}${url.search}`, request);
        restored.headers.delete("forwarded");

        return restored;
    }

    /** Forward a request for a name to the destination the directory's replica resolves. */
    async #forward(request: Request, isForwarded: boolean): Promise<Response> {
        // resolve the name's destination
        const name = new URL(request.url).hostname;
        const destination = await this.#resolve(name);

        // refuse a forwarded request for a machine tunnelled elsewhere, which its relay resolves again
        if (isForwarded && destination.kind !== "tunnel") {
            throw RelayError.of("RELAY_OTHER", `${name} is tunnelled through another relay`);
        }

        return this.#send(destination, request);
    }

    /** Forward a request to a machine through its tunnel (RFC 7239). */
    async #send(destination: Destination, request: Request): Promise<Response> {
        // call a machine through its tunnel, handing on the client's head alone
        if (destination.kind === "tunnel") {
            const headers = new Headers({
                [FORWARDED_HEAD_HEADER]: ForwardedRequest.head(request),
            });

            return this.call(destination.machineId, "forward", {
                method: request.method,
                headers,
                body: request.body,
                ...(request.body === null ? {} : { duplex: "half" }),
            });
        }

        // send the path, query and streamed body to the endpoint's origin
        const url = new URL(request.url);
        const target = new URL(`${url.pathname}${url.search}`, destination.url);
        const forwarded = new Request(target.href, request);
        forwarded.headers.set("forwarded", `host="${url.host}";proto=${url.protocol.slice(0, -1)}`);

        // refuse an endpoint the network cannot reach as a bad gateway
        try {
            return await this.#fetch(forwarded);
        } catch (error) {
            throw RelayError.of("ENDPOINT_UNREACHABLE", `${destination.url} does not answer`, {
                cause: error,
            });
        }
    }

    /** Find where a name's requests go through the replicas of the directory and the machines. */
    async #resolve(name: string): Promise<Destination> {
        const origin = InstallationOrigin.parse(name, this.domains.space);
        const computer = MachineAddress.parse(name, this.domains.machine);

        // route a space's name, and its branches', to its placement's machine
        if (origin !== undefined) {
            return this.#locateSpace(name, SpaceAddress.of(origin));
        }
        // route a machine's name to its tunnel
        else if (computer !== undefined) {
            return this.#locateMachine(name, computer.handle, computer.machine);
        }
        // resolve nothing else
        else {
            throw RelayError.of("NAME_UNRESOLVED", `${name} does not resolve`);
        }
    }

    /** Find the machine serving a space by its address through the directory's replica. */
    async #locateSpace(name: string, address: SpaceAddress): Promise<Destination> {
        // find the space's placement through the replicated claims
        const named = await this.#directory.find(space, address);
        const [placed] = named === undefined ? [] : await Placement.find(this.#view, [named.id]);
        if (placed === undefined) {
            throw RelayError.of("NAME_UNRESOLVED", `${name} does not resolve`);
        }

        return this.#destination(placed.machine);
    }

    /** Find a standing machine of any residency by its name within its account. */
    async #locateMachine(name: string, handle: string, machineName: string): Promise<Destination> {
        // find the account by its handle, and its standing machine by name
        const accountId = await this.#directory.account(handle);
        const [named] =
            accountId === undefined
                ? []
                : await this.#view.read(machine.table).findMany({
                      where: {
                          scope: accountId,
                          name: machineName,
                          revokedAt: { isNull: true },
                      },
                  });
        if (named === undefined) {
            throw RelayError.of("NAME_UNRESOLVED", `${name} does not resolve`);
        }

        return this.#destination(named.id);
    }

    /** Find where a machine's requests go. */
    async #destination(machineId: Identifier<"machine">): Promise<Destination> {
        // refuse a machine without an answering endpoint
        const published = await this.#view
            .read(endpoint.table)
            .findFirst({ where: { id: machineId } });
        if (published === undefined || !Endpoint.isAnswering(published, this.#clock.now())) {
            throw RelayError.of("MACHINE_DISCONNECTED", `machine ${machineId} is disconnected`);
        }

        // route through its tunnel here, else to the relay with its tunnel or its own URL
        return this.isHere(published)
            ? { kind: "tunnel", machineId }
            : { kind: "endpoint", url: published.relay?.url ?? published.url };
    }

    /** Read a replicated machine's name, its account and whether it was revoked. */
    async #machine(machineId: Identifier<"machine">): Promise<
        | {
              readonly name: string;
              readonly account: Identifier<"account">;
              readonly isRevoked: boolean;
          }
        | undefined
    > {
        // read the machine and the handle its account claims, none for a machine outside accounts
        const replicated = await this.#view
            .read(machine.table)
            .findFirst({ where: { id: machineId } });
        const handle =
            replicated === undefined ? undefined : await this.#directory.handle(replicated.scope);
        if (replicated === undefined || handle === undefined) {
            return undefined;
        }

        // read the machine's name and whether it was revoked
        const address = { machine: replicated.name, handle };

        return {
            name: MachineAddress.format(address, this.domains.machine),
            account: schema.identifier("account").parse(replicated.scope),
            isRevoked: replicated.revokedAt !== null,
        };
    }

    /** Report whether a machine has a standing key, unrevoked, unsuspended and unexpired. */
    async #isKeyed(machineId: Identifier<"machine">, now: number): Promise<boolean> {
        const standing = await this.#view.read(key.table).findFirst({
            where: {
                parentType: machine.name,
                parentId: machineId,
                revokedAt: { isNull: true },
                suspendedAt: { isNull: true },
                expiresAt: { gt: now },
            },
        });

        return standing !== undefined;
    }
}

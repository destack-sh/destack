import {
    TUNNEL_METADATA_PATH,
    TUNNEL_PATH,
    type TunnelMetadata,
    TunnelProtocol,
} from "@destack/tunnel";
import {
    type UniverseOrigin,
    InstallationOrigin,
    MachineAddress,
    S3Origin,
    SpaceAddress,
} from "@destack/account/address";
import { Directory } from "@destack/account/client";
import {
    account,
    endpoint,
    Endpoint,
    key,
    machine,
    Placement,
    type RegionCode,
    space,
} from "@destack/account/object";
import type { Projection, Snapshot } from "@destack/db/database";
import { type Clock, type Environment, Forwarded, WebSocketUpgrade } from "@destack/environment";
import { Failure } from "@destack/error";
import { type Identifier, schema } from "@destack/schema";
import { Claim, claimTable, type Fetch, PackageMount, refusal } from "@destack/object";
import { CLIENT_HEAD_HEADER, ClientHead, PARTITION_HEADER } from "@destack/object/workload";
import { RelayError } from "../error/index.ts";
import { PackageId } from "@destack/package";
import relayPackage from "../package.ts";
import {
    HANDSHAKE_HEADER,
    type Handshake,
    STANDING_ERRORS,
    Tunnel,
    type TunnelCloseReason,
    type TunnelStanding,
    TUNNELS_PATH,
    type TunnelCall,
} from "./tunnel.ts";
import { TunnelToken } from "./token.ts";

/** What a relay routes by, and where. */
export interface RelayOptions {
    /** The relay's own origin, where machines open their tunnels. */
    readonly origin: string;
    /** The universe, whose issuer signs machines' tokens and whose domains names resolve under. */
    readonly universe: UniverseOrigin;
    /** The relay installation itself, which calls each machine's tunnel in its partition. */
    readonly itself: { readonly url: string; readonly fetch: Fetch };
    /** The replica of the universe's directory, identities and regions. */
    readonly view: Snapshot;
    /** Call managed machines at their published endpoints. */
    readonly fetch: Fetch;
    /** The clock tokens and endpoints lapse on, and the WebCrypto verifying the tokens. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "network" | "telemetry">;
    /** Wait until the replicas of the directory, the identities and the regions synced. */
    readonly synced: () => Promise<void>;
    /** The region whose accounts' machines tunnel to the relay. */
    readonly region: RegionCode;
}

/** Where the relay forwards a name's requests: a machine's tunnel, or a published endpoint. */
export type Destination =
    | { readonly kind: "tunnel"; readonly machineId: Identifier<"machine"> }
    | { readonly kind: "endpoint"; readonly url: string };

/** The edge of Destack's names. */
export class Relay {
    /** The universe names resolve in. */
    readonly universe: UniverseOrigin;

    /** The replica of the directory and the identities: placements, machines and keys. */
    readonly #view: Snapshot;
    /** Wait until the replicas that handshakes and standings read synced. */
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

    /** Call managed machines. */
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

        // set the token verifier and the calls to tunnels and managed machines
        this.#tokens = new TunnelToken(
            options.universe.issuer,
            options.region,
            this.#view,
            options.environment,
        );
        this.universe = options.universe;
        this.#itself = options.itself;
        this.#origin = new URL(options.origin);
        this.#fetch = options.fetch;
    }

    /** The URL machines open their tunnels at. */
    get url(): string {
        return new URL(TUNNEL_PATH, this.#origin).href;
    }

    /** Answer its tunnel's metadata, open a machine's tunnel, and relay every other request. */
    async fetch(request: Request): Promise<Response> {
        // answer the tunnel's metadata, naming the relay's package as the resource its tokens are for
        const url = new URL(request.url);
        const isOwn = url.host === this.#origin.host;
        if (isOwn && url.pathname === TUNNEL_METADATA_PATH) {
            const metadata: TunnelMetadata = { resource: PackageId.resource(relayPackage.id) };

            return Response.json(metadata);
        }
        // forward requests for names
        else if (!isOwn || url.pathname !== TUNNEL_PATH) {
            return this.#relay(request);
        }

        // accept the machine's handshake and open its tunnel while the request lasts
        try {
            const handshake = await this.handshake(request);

            return await this.call(handshake.machineId, "open", {
                method: "POST",
                headers: { [HANDSHAKE_HEADER]: JSON.stringify(handshake) },
                signal: request.signal,
            });
        } catch (error) {
            // rethrow once the client or the server aborted the request
            if (request.signal.aborted) {
                throw error;
            }

            return refusal(error, this.#telemetry);
        }
    }

    /** Accept a machine's opening handshake with the token it offers as a WebSocket subprotocol. */
    async handshake(request: Request): Promise<Handshake> {
        // require a machine's token
        const upgrade = WebSocketUpgrade.of(request);
        const token = upgrade === undefined ? undefined : TunnelProtocol.token(upgrade);
        if (token === undefined) {
            throw RelayError.of(
                "SUBPROTOCOL_UNAUTHENTICATED",
                "authentication required: the bearer subprotocol has no token",
            );
        }

        // verify the token for the relay's region by the universe's keys once their replica synced
        await this.#synced();
        const { machineId, lapsesAt, kid } = await this.#tokens.verify(token, this.#clock.now());

        // require the machine to stand with a standing key
        const standing = await this.standing(machineId);
        if ("reason" in standing) {
            const [reason, refused] = STANDING_ERRORS[standing.reason];
            throw RelayError.of(reason, `machine ${machineId} ${refused}`);
        }

        return { machineId, lapsesAt, kid, name: standing.name };
    }

    /** Report whether an endpoint leads to a tunnel at this relay. */
    isHere(published: Pick<Endpoint, "relay">): boolean {
        return published.relay === this.#origin.origin;
    }

    /** Read whether a machine stands with a standing key: its name, or why its tunnel ends. */
    async standing(machineId: Identifier<"machine">): Promise<TunnelStanding> {
        // wait for the synced replicas, then read the machine and its keys
        await this.#synced();
        const replicated = await this.#machine(machineId);
        if (replicated === undefined) {
            return { reason: "machine-unknown" };
        } else if (!(await this.#isKeyed(machineId, this.#clock.now()))) {
            return { reason: "key-revoked" };
        }

        return { name: replicated.name };
    }

    /** Bring a machine's tunnel here in line with its replica: its name, or its close. */
    async reconcile(machineId: Identifier<"machine">): Promise<void> {
        // read whether the machine stands, and where its endpoint leads
        const standing = await this.standing(machineId);
        const published = await this.#view
            .read(endpoint.table)
            .findFirst({ where: { parentId: machineId } });

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

    /** Project the tunnels here from the replica, telling each machine its name or ending it. */
    projection(): Projection {
        // key each tunnel by its machine, among the endpoints here
        const here = { relay: this.#origin.origin };

        return {
            name: "tunnel",
            key: "parentId",
            result: "process",
            all: { table: endpoint.table, column: "parentId", where: here },
            reads: [
                // take a machine whose endpoint came here or left
                { table: endpoint.table, fields: ["relay"], where: here },
                // take a renamed machine tunnelled here
                { table: machine.table, fields: ["name"], on: { parentId: "id" } },
                // take the machine of a key that was revoked, suspended, restored or removed
                {
                    table: key.table,
                    fields: ["revokedAt", "suspendedAt"],
                    where: { parentType: machine.name },
                    on: { parentId: "parentId" },
                },
                // take the machines of an account whose handle claim changed
                {
                    table: claimTable,
                    fields: ["state"],
                    where: { index: Claim.index(account, "handle") },
                    path: [
                        {
                            table: machine.table,
                            cardinality: "many",
                            on: { kind: "key", column: "scope", parent: "objectId" },
                        },
                    ],
                    on: { parentId: "id" },
                },
            ],
            project: (machineId) => this.reconcile(schema.identifier("machine").parse(machineId)),
        };
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
            // rethrow once the client or the server aborted the request
            if (request.signal.aborted) {
                throw error;
            }

            return refusal(error, this.#telemetry);
        }
    }

    /** Restore a request another relay forwarded to this relay's origin (RFC 7239). */
    #restore(request: Request): Request | undefined {
        // take a request at the relay's origin with a Forwarded header
        const url = new URL(request.url);
        if (url.host !== this.#origin.host || !request.headers.has("forwarded")) {
            return undefined;
        }

        // require the forwarded host and protocol
        const { for: client, host, proto } = Forwarded.first(request);
        if (host === undefined || proto === undefined) {
            throw RelayError.of(
                "HOST_REQUIRED",
                "a forwarded host and protocol is required: the forwarded request has no host or protocol",
            );
        }

        // ask for the name as the client did, keeping the client a front door wrote
        const restored = new Request(`${proto}://${host}${url.pathname}${url.search}`, request);
        restored.headers.delete("forwarded");

        return client === undefined ? restored : Forwarded.replace(restored, { for: client });
    }

    /** Forward a request for a name to the destination the directory's replica resolves. */
    async #forward(request: Request, isForwarded: boolean): Promise<Response> {
        // resolve the name's destination
        const name = new URL(request.url).hostname;
        const destination = await this.#resolve(name);

        // refuse a forwarded request for a machine tunnelled elsewhere, which its relay resolves again
        if (isForwarded && destination.kind !== "tunnel") {
            throw RelayError.of("TUNNEL_MOVED", `tunnel of '${name}' moved to another relay`);
        }

        return this.#send(destination, request);
    }

    /** Forward a request to a machine through its tunnel (RFC 7239). */
    async #send(destination: Destination, request: Request): Promise<Response> {
        // call a machine through its tunnel, handing on the client's head alone
        if (destination.kind === "tunnel") {
            const headers = new Headers({
                [CLIENT_HEAD_HEADER]: ClientHead.write(request),
            });

            return this.call(destination.machineId, "forward", {
                method: request.method,
                headers,
                body: request.body,
                ...(request.body === null ? {} : { duplex: "half" }),
                signal: request.signal,
            });
        }

        // send the path, query and streamed body to the endpoint's origin
        const url = new URL(request.url);
        const target = new URL(`${url.pathname}${url.search}`, destination.url);
        const forwarded = Forwarded.replace(new Request(target.href, request), {
            ...Forwarded.first(request),
            host: url.host,
            proto: url.protocol.slice(0, -1),
        });

        // refuse an endpoint the network cannot reach as a bad gateway
        try {
            return await this.#fetch(forwarded);
        } catch (error) {
            throw RelayError.of("ENDPOINT_UNREACHABLE", `could not reach '${destination.url}'`, {
                cause: error,
            });
        }
    }

    /** Find where a name's requests go through the replicas of the directory and the machines. */
    async #resolve(name: string): Promise<Destination> {
        // read the name as a space's origin, a space's S3 origin or a machine's name
        const origin = InstallationOrigin.parse(name, this.universe.domains.space);
        const s3 = S3Origin.parse(name, this.universe.domains.space);
        const computer = MachineAddress.parse(name, this.universe.domains.machine);

        // route a space's name, and its branches', to its placement's machine
        if (origin !== undefined) {
            return this.#locateSpace(name, SpaceAddress.of(origin));
        }
        // route a space's S3 origin to its placement's machine
        else if (s3 !== undefined) {
            return this.#locatePlacement(name, s3);
        }
        // route a machine's name to its tunnel
        else if (computer !== undefined) {
            return this.#locateMachine(name, computer.handle, computer.machine);
        }
        // resolve nothing else
        else {
            throw RelayError.of("NAME_NOT_FOUND", `name '${name}' not found`);
        }
    }

    /** Find the machine serving a space by its address through the directory's replica. */
    async #locateSpace(name: string, address: SpaceAddress): Promise<Destination> {
        // find the space through the replicated claims, then its placement
        const named = await this.#directory.find(space, address);
        if (named === undefined) {
            throw RelayError.of("NAME_NOT_FOUND", `name '${name}' not found`);
        }

        return this.#locatePlacement(name, named.id);
    }

    /** Find the machine a space is placed at through the directory's replica. */
    async #locatePlacement(name: string, spaceId: Identifier<"space">): Promise<Destination> {
        // refuse a space placed nowhere
        const [placed] = await Placement.find(this.#view, [spaceId]);
        if (placed === undefined) {
            throw RelayError.of("NAME_NOT_FOUND", `name '${name}' not found`);
        }

        return this.#destination(placed.machine);
    }

    /** Find a standing machine of any region by its name within its account. */
    async #locateMachine(name: string, handle: string, machineName: string): Promise<Destination> {
        // find the account by its handle, and its live machine by name
        const accountId = await this.#directory.account(handle);
        const [named] =
            accountId === undefined
                ? []
                : await this.#view
                      .read(machine.table)
                      .findMany({ where: machine.address(accountId, machineName) });
        if (named === undefined) {
            throw RelayError.of("NAME_NOT_FOUND", `name '${name}' not found`);
        }

        return this.#destination(named.id);
    }

    /** Find where a machine's requests go. */
    async #destination(machineId: Identifier<"machine">): Promise<Destination> {
        // refuse a machine without an answering endpoint, by its name while the directory has it
        const published = await this.#view
            .read(endpoint.table)
            .findFirst({ where: { parentId: machineId } });
        if (published === undefined || !Endpoint.isAnswering(published, this.#clock.now())) {
            const named = await this.#machine(machineId);
            throw RelayError.of(
                "MACHINE_UNAVAILABLE",
                named === undefined
                    ? `machine '${machineId}' is unavailable: it is not in the directory`
                    : `'${named.name}' is unavailable: it is disconnected`,
            );
        }

        // route through its tunnel here, else to the relay with its tunnel or its own URL
        return this.isHere(published)
            ? { kind: "tunnel", machineId }
            : { kind: "endpoint", url: published.relay ?? published.url };
    }

    /** Read a live machine's name and its account, absent once deleted. */
    async #machine(machineId: Identifier<"machine">): Promise<
        | {
              readonly name: string;
              readonly account: Identifier<"account">;
          }
        | undefined
    > {
        // read the live machine and the handle its account claims, none for a machine outside accounts
        const replicated = await this.#view
            .read(machine.table)
            .findFirst({ where: { id: machineId, deletedAt: { isNull: true } } });
        const handle =
            replicated === undefined ? undefined : await this.#directory.handle(replicated.scope);
        if (replicated === undefined || handle === undefined) {
            return undefined;
        }

        // read the machine's name
        const address = { machine: replicated.name, handle };

        return {
            name: MachineAddress.format(address, this.universe.domains.machine),
            account: schema.identifier("account").parse(replicated.scope),
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

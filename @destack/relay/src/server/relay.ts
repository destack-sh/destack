import {
    AddressResolver,
    DOMAINS,
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
    type Machine,
    machine,
    Placement,
    space,
} from "@destack/account/object";
import { and, eq, gt, isNull, Snapshot } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import type { Identifier } from "@destack/schema";
import type { Fetch } from "@destack/runtime";
import type { TokenVerifier } from "@destack/runtime/authentication";
import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import type { Call } from "@destack/sync";
import { refusal, reportError } from "@destack/runtime/server";
import {
    FORWARDED_URL_HEADER,
    type ServerContext,
    PARTITION_HEADER,
} from "@destack/runtime/workload";
import { PackageMount } from "@destack/runtime";
import { tunnel } from "../object/index.ts";
import relayPackage from "../package.ts";
import { TunnelProtocol } from "../session/index.ts";
import { HANDSHAKE_HEADER, type Handshake, Tunnel, TUNNEL_PATH, TUNNELS_PATH } from "./tunnel.ts";

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
    /** The relay installation itself, which calls each machine's tunnel in its partition. */
    readonly itself: ReturnType<ServerContext["at"]>;
    /** The relay installation's store, with a row per tunnelled machine and the replicas. */
    readonly store: Pick<ObjectStore, "principal" | "self" | "clock" | "database" | "mutate">;
    /** The domains names resolve under. */
    readonly domains?: Domains;
    /** Call cloud machines at their published endpoints. */
    readonly fetch: Fetch;
}

/** Where the relay forwards a name's requests: a machine's tunnel, or a published endpoint. */
export type Destination =
    | { readonly kind: "tunnel"; readonly machineId: Identifier<"machine"> }
    | { readonly kind: "endpoint"; readonly url: string };

/** The edge of Destack's names. */
export class Relay {
    /** The domains names resolve under. */
    readonly domains: Domains;

    /** The directory's replica: placements, machines, endpoints and claims of names. */
    readonly #directory: Snapshot;

    /** The resolver of handles and names over the directory's replica. */
    readonly #names: AddressResolver;

    /** The relay installation itself, which calls each machine's tunnel in its partition. */
    readonly #itself: ReturnType<ServerContext["at"]>;

    /** The relay installation's store, with a row per tunnelled machine and the replicas. */
    readonly #store: RelayOptions["store"];

    /** Verify the universe's tokens machines open their tunnels with. */
    readonly #tokens: TokenVerifier;

    /** The relay's own origin. */
    readonly #origin: URL;

    /** Call cloud machines. */
    readonly #fetch: Fetch;

    /** Route as the options describe. */
    constructor(options: RelayOptions) {
        // resolve handles and names through the replicas in the store's database
        this.#store = options.store;
        this.#directory = Snapshot.live(options.store.database);
        this.#names = new AddressResolver(this.#directory);

        // set the token verifier and the calls to tunnels and cloud machines
        this.#tokens = Tunnel.verifier(options.issuer, this.#directory);
        this.domains = options.domains ?? DOMAINS;
        this.#itself = options.itself;
        this.#origin = new URL(options.origin);
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
            const handshake = await this.handshake(request, this.#store.clock.now());

            return await this.tunnel(handshake.machineId, "open", {
                method: "POST",
                headers: { [HANDSHAKE_HEADER]: JSON.stringify(handshake) },
            });
        } catch (error) {
            const reported = reportError(error);

            return new Response(null, { status: reported.status });
        }
    }

    /** Accept a machine's opening handshake with the token it offers as a WebSocket subprotocol. */
    async handshake(request: Request, now: number): Promise<Handshake> {
        // require a machine's token
        const token = TunnelProtocol.token(request.headers.get("sec-websocket-protocol"));
        if (token === undefined) {
            throw new CallError("UNAUTHORIZED", { message: "invalid bearer subprotocol" });
        }
        const { machineId, lapsesAt } = await Tunnel.verify(this.#tokens, token, now);

        // require the machine to stand with a standing key
        const replicated = await this.#machine(machineId);
        if (replicated === undefined || replicated.isRevoked) {
            throw new CallError("UNAUTHORIZED", { message: `machine ${machineId} is revoked` });
        } else if (!(await this.#isKeyed(machineId, now))) {
            throw new CallError("UNAUTHORIZED", { message: `machine ${machineId} lacks a key` });
        }
        const { name } = replicated;

        // open the tunnel under its name, or rename a reopened one, which the machine publishes at each renewal
        await this.#upsertTunnel(machineId, name);

        return { machineId, lapsesAt, name };
    }

    /** List the standing machines of an account. */
    async machines(accountId: Identifier<"account">): Promise<readonly Identifier<"machine">[]> {
        const rows = await this.#store.database
            .select({ id: machine.table.id })
            .from(machine.table)
            .where(and(eq(machine.table.scope, accountId), isNull(machine.table.revokedAt)));

        return rows.map((row) => row.id);
    }

    /** Bring a machine's tunnel in line with its replica. */
    async reconcile(machineId: Identifier<"machine">): Promise<void> {
        // leave a machine the replicas no longer have, and one without a tunnel
        const replicated = await this.#machine(machineId);
        const opened = await this.#opened(machineId);
        if (replicated === undefined || opened === undefined) {
            return;
        }

        // read whether the machine stands with a standing key
        const { name, isRevoked } = replicated;
        const target = tunnel.target({
            scope: this.#store.principal.scope,
            id: Tunnel.id(machineId),
        });
        const now = this.#store.clock.now();
        const isStanding = !isRevoked && (await this.#isKeyed(machineId, now));

        // close the tunnel of a revoked or keyless machine
        let call: Call;
        if (!isStanding) {
            const reason = isRevoked ? "machine-revoked" : "key-revoked";
            call = tunnel.call("close", { ...target, reason });
        }
        // rename the tunnel of a renamed machine, which tells the machine
        else if (opened.name !== name) {
            call = tunnel.call("rename", { ...target, name });
        }
        // leave a tunnel under its name
        else {
            return;
        }

        // call it as the relay installation
        await this.#store.mutate(this.#store.self, { id: RequestId.create(now), calls: [call] });
    }

    /** Call a machine's tunnel in its partition through the relay installation itself. */
    tunnel(
        machineId: Identifier<"machine">,
        call: string,
        options: RequestInit,
    ): Promise<Response> {
        // address the machine's tunnel partition
        const headers = new Headers(options.headers);
        headers.set(PARTITION_HEADER, Tunnel.id(machineId));
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
            return refusal(error);
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
            throw new CallError("BAD_REQUEST", {
                message: "the forwarded request lacks a host or protocol",
            });
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
            throw new CallError("MISDIRECTED_REQUEST", {
                message: `${name} is tunnelled through another relay`,
            });
        }

        return this.#send(destination, request);
    }

    /** Forward a request to a machine through its tunnel (RFC 7239). */
    async #send(destination: Destination, request: Request): Promise<Response> {
        // call a machine through its tunnel
        if (destination.kind === "tunnel") {
            const headers = new Headers(request.headers);
            headers.set(FORWARDED_URL_HEADER, request.url);

            return this.tunnel(destination.machineId, "forward", {
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

        return this.#fetch(forwarded);
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
            throw new CallError("NOT_FOUND", { message: `${name} resolves to nothing` });
        }
    }

    /** Find the machine serving a space by its address through the directory's replica. */
    async #locateSpace(name: string, address: SpaceAddress): Promise<Destination> {
        // find the space's placement through the replicated claims
        const named = await this.#names.find(space, address);
        const [placed] =
            named === undefined ? [] : await Placement.find(this.#directory, [named.id]);
        if (placed === undefined) {
            throw new CallError("NOT_FOUND", { message: `${name} resolves to nothing` });
        }

        return this.#destination(placed.machine);
    }

    /** Find a standing machine of any residency by its name within its account. */
    async #locateMachine(name: string, handle: string, machineName: string): Promise<Destination> {
        // find the account by its handle, and its standing machine by name
        const accountId = await this.#names.account(handle);
        const [named] =
            accountId === undefined
                ? []
                : await this.#directory.read(machine.table).findMany({
                      where: {
                          scope: accountId,
                          name: machineName,
                          revokedAt: { isNull: true },
                      },
                  });
        if (named === undefined) {
            throw new CallError("NOT_FOUND", { message: `${name} resolves to nothing` });
        }

        return this.#destination(named.id);
    }

    /** Find where a machine's requests go. */
    async #destination(machineId: Identifier<"machine">): Promise<Destination> {
        // route to a person's computer through its tunnel here
        const replicated = await this.#machine(machineId);
        if (replicated?.kind === "personal" && (await this.#opened(machineId)) !== undefined) {
            return { kind: "tunnel", machineId };
        }

        // route to a cloud machine at its URL, or to the relay with a computer's tunnel, unless that is this relay
        const published = await this.#directory
            .read(endpoint.table)
            .findFirst({ where: { id: machineId } });
        const url = published?.relay?.url ?? published?.url;
        const isLostHere = url !== undefined && new URL(url).host === this.#origin.host;
        if (
            published === undefined ||
            url === undefined ||
            isLostHere ||
            !Endpoint.isAnswering(published, this.#store.clock.now())
        ) {
            throw new CallError("SERVICE_UNAVAILABLE", {
                message: `machine ${machineId} is disconnected`,
            });
        }

        return { kind: "endpoint", url };
    }

    /** Read a replicated machine's kind, name and whether it was revoked. */
    async #machine(machineId: Identifier<"machine">): Promise<
        | {
              readonly kind: Machine["kind"];
              readonly name: string;
              readonly isRevoked: boolean;
          }
        | undefined
    > {
        // read the machine with its account's handle
        const [replicated] = await this.#store.database
            .select({
                machine: machine.table.name,
                kind: machine.table.kind,
                revokedAt: machine.table.revokedAt,
                handle: account.table.handle,
            })
            .from(machine.table)
            .innerJoin(account.table, eq(account.table.id, machine.table.scope))
            .where(eq(machine.table.id, machineId));
        if (replicated === undefined) {
            return undefined;
        }

        // read the machine's name and whether it was revoked
        return {
            kind: replicated.kind,
            name: MachineAddress.format(replicated, this.domains.machine),
            isRevoked: replicated.revokedAt !== null,
        };
    }

    /** Open a machine's tunnel under the name the relay routes to it, or rename a reopened one. */
    async #upsertTunnel(machineId: Identifier<"machine">, name: string): Promise<void> {
        // open a new tunnel, rename an open one under another name, and leave one under its name
        const target = tunnel.target({
            scope: this.#store.principal.scope,
            id: Tunnel.id(machineId),
        });
        const opened = await this.#opened(machineId);
        if (opened?.name === name) {
            return;
        }
        const call =
            opened === undefined
                ? tunnel.call("open", { ...target, name })
                : tunnel.call("rename", { ...target, name });

        // call it as the relay installation
        const now = this.#store.clock.now();
        await this.#store.mutate(this.#store.self, {
            id: RequestId.create(now),
            calls: [call],
        });
    }

    /** Read a machine's open tunnel row, absent before its first open. */
    async #opened(machineId: Identifier<"machine">) {
        return this.#directory
            .read(tunnel.table)
            .findFirst({ where: { id: tunnel.identifier(Tunnel.id(machineId)) } });
    }

    /** Report whether a machine has a standing key, unrevoked, unsuspended and unexpired. */
    async #isKeyed(machineId: Identifier<"machine">, now: number): Promise<boolean> {
        const [standing] = await this.#store.database
            .select({ id: key.table.id })
            .from(key.table)
            .where(
                and(
                    eq(key.table.parentType, machine.name),
                    eq(key.table.parentId, machineId),
                    isNull(key.table.revokedAt),
                    isNull(key.table.suspendedAt),
                    gt(key.table.expiresAt, now),
                ),
            )
            .limit(1);

        return standing !== undefined;
    }
}

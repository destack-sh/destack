import { AccessContext, Authorization, Caller } from "@destack/access";
import {
    DOMAINS,
    type Domains,
    InstallationOrigin,
    MachineAddress,
    SpaceAddress,
} from "@destack/account/address";
import { DatabaseDirectory, HandleResolver } from "@destack/account/directory";
import { account, directoryShape, key, Machine, machine, space } from "@destack/account/object";
import { AuditRecorder, Journal } from "@destack/audit/server";
import {
    and,
    Change,
    type DatabaseConnection,
    eq,
    gt,
    isNull,
    TABLE,
    type Table,
    Transaction,
} from "@destack/db";
import { Call } from "@destack/object";
import { DatabaseClaimStore, type ObjectStore } from "@destack/object/server";
import type {} from "@destack/package/import-meta";
import { type Identifier, present, schema } from "@destack/schema";
import type { Fetch } from "@destack/service";
import type { TokenVerifier } from "@destack/service/authentication";
import type { Controller, Reconciliation } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { type CallKey, RequestId } from "@destack/service/request";
import { refusal, reportError } from "@destack/service/server";
import {
    FORWARDED_URL_HEADER,
    type InstallationContext,
    PARTITION_HEADER,
} from "@destack/service/workload";
import { ServiceMount } from "@destack/service";
import { Scope, type Subscription, type Uplink } from "@destack/sync";
import type * as sync from "@destack/sync";
import { tunnelClose, tunnelOpen } from "../audit/index.ts";
import { tunnel } from "../object/index.ts";
import { relayService } from "../service/index.ts";
import { TunnelProtocol } from "../session/index.ts";
import type { DestinationCache } from "./destination.ts";
import {
    ADMISSION_HEADER,
    type Admission,
    type MachineDestination,
    Tunnel,
    TUNNEL_PATH,
    TUNNELS_PATH,
} from "./tunnel.ts";

/** The relay's package, the audience of the tokens machines open their tunnels with. */
export const RELAY_PACKAGE = import.meta.destack.package;

/** The lifetime of the tokens machines renew their tunnels with: ten minutes. */
export const RELAY_TOKEN_LIFETIME_MILLISECONDS = 10 * 60_000;

/** The host a relay names in the Forwarded header of a request it forwards (RFC 7239). */
const FORWARDED_HOST = /host="?([^";,]+)"?/u;

/** The answer of a tunnel's close: whether a connection was open. */
const Closing = schema.object({ closed: schema.boolean() });

/** The protocol a relay names in the Forwarded header of a request it forwards (RFC 7239). */
const FORWARDED_PROTO = /proto=([a-z]+)/u;

/** What a relay routes by, and where. */
export interface RelayOptions {
    /** The relay's own origin, where machines open their tunnels. */
    readonly origin: string;
    /** The account service's uplink, used as the relay installation. */
    readonly publisher: Uplink;
    /** Verify the universe's tokens machines open tunnels with, for the relay's package. */
    readonly tokens: TokenVerifier;
    /** The key the relay's journal fingerprints sensitive values under. */
    readonly callKey: CallKey;
    /** The relay installation itself, which reaches each machine's tunnel in its partition. */
    readonly itself: ReturnType<InstallationContext["at"]>;
    /** The relay installation's store, keeping a row per tunnelled machine and the copies. */
    readonly store: Pick<ObjectStore, "principal" | "clock" | "database" | "authorizer">;
    /** Report a failure the relay answers to no one. */
    readonly report: (error: unknown) => void;
    /** The domains names resolve under. */
    readonly domains?: Domains;
    /** Call cloud machines at their published endpoints. */
    readonly fetch?: Fetch;
    /** The destinations the relay keeps across requests. */
    readonly destinations?: DestinationCache;
}

/** Where the relay forwards a name's requests. */
export type Destination = MachineDestination | { readonly endpoint: string };

/** The edge of Destack's names. */
export class Relay {
    /** Verify the universe's tokens machines open and renew their tunnels with. */
    readonly tokens: TokenVerifier;
    /** The domains names resolve under. */
    readonly domains: Domains;
    /** Report a failure the relay answers to no one. */
    readonly report: (error: unknown) => void;
    /** The journal recording tunnel openings and closes. */
    readonly journal: Journal;
    /** The account service's uplink, which publishes the relay's tunnelled machines' endpoints. */
    readonly #publisher: Uplink;
    /** The directory over the copied placements and machines. */
    readonly #directory: DatabaseDirectory;
    /** The claims over the copied claims of names. */
    readonly #claims: DatabaseClaimStore;
    /** The relay installation itself, which reaches each machine's tunnel in its partition. */
    readonly #itself: ReturnType<InstallationContext["at"]>;
    /** The relay installation's store, keeping a row per tunnelled machine and the copies. */
    readonly #store: RelayOptions["store"];
    /** The relay's own origin. */
    readonly #origin: URL;
    /** Call cloud machines. */
    readonly #fetch: Fetch;
    /** The destinations kept across requests. */
    readonly #destinations: DestinationCache | undefined;

    /** Route as the options describe. */
    constructor(options: RelayOptions) {
        // resolve handles and names through the copies in the store's database
        const { database, clock } = options.store;
        this.#store = options.store;
        this.#directory = new DatabaseDirectory(database, clock);
        this.#claims = new DatabaseClaimStore(database, clock);
        this.journal = new Journal(database, options.callKey, { clock });

        // keep the token verifier and the calls to the account service, tunnels and cloud machines
        this.tokens = options.tokens;
        this.domains = options.domains ?? DOMAINS;
        this.report = options.report;
        this.#publisher = options.publisher;
        this.#itself = options.itself;
        this.#origin = new URL(options.origin);
        this.#fetch = options.fetch ?? globalThis.fetch;
        this.#destinations = options.destinations;
    }

    /** Ask the relay's root to withdraw the endpoint of a machine left without a connection. */
    async withdrawAtRoot(machineId: Identifier<"machine">): Promise<void> {
        const url = `${this.#itself.url}${ServiceMount.path(RELAY_PACKAGE.id)}${TUNNELS_PATH}/${machineId}/withdraw`;
        const response = await this.#itself.fetch(new Request(url, { method: "POST" }));
        await requireAnswer(response, `withdraw machine ${machineId}`);
    }

    /** Call a machine's tunnel in its partition through the relay installation itself. */
    #tunnel(
        destination: MachineDestination,
        call: string,
        options: RequestInit,
    ): Promise<Response> {
        // address the machine's tunnel partition
        const headers = new Headers(options.headers);
        headers.set(PARTITION_HEADER, Tunnel.id(destination.machineId));
        const path = `${ServiceMount.path(RELAY_PACKAGE.id)}${TUNNELS_PATH}`;
        const url = `${this.#itself.url}${path}/${destination.machineId}/${call}`;

        return this.#itself.fetch(new Request(url, { ...options, headers }));
    }

    /** Request the copy of the directory a relay installation keeps below its space. */
    static subscription(below: string): Subscription {
        return directoryShape.subscription({
            name: `${directoryShape.name}:${below}`,
            scope: Scope.universe.id,
            below,
            parameters: {},
        });
    }

    /** Keep the reads of the copies until the signal aborts. */
    async follow(signal: AbortSignal): Promise<void> {
        await Promise.all([this.#directory.follow(signal), this.#claims.follow(signal)]);
    }

    /** The URL machines open their tunnels at. */
    get url(): string {
        return new URL(TUNNEL_PATH, this.#origin).href;
    }

    /** Admit a machine opening its tunnel at the relay's own origin. */
    async route(request: Request): Promise<Response> {
        // forward requests for names
        const url = new URL(request.url);
        const isTunnel = url.host === this.#origin.host && url.pathname === TUNNEL_PATH;
        if (!isTunnel) {
            return this.fetch(request);
        }

        // admit the machine by its offered token and open its tunnel, answering a refusal's status alone
        try {
            const admission = await this.admit(request, this.#store.clock.now());

            return await this.#tunnel(admission, "open", {
                method: "POST",
                headers: { [ADMISSION_HEADER]: JSON.stringify(admission) },
            });
        } catch (error) {
            const reported = reportError(error);

            return new Response(null, { status: reported.status });
        }
    }

    /** Bring a machine's tunnel in line with its copy. */
    async reconcile(machineId: Identifier<"machine">): Promise<void> {
        // leave a machine the copies no longer keep
        const copied = await this.#machine(machineId);
        if (copied === undefined) {
            return;
        }

        // tell a standing machine with a standing key its name
        const { destination, name } = copied;
        if (name !== undefined && (await this.#isKeyed(machineId, this.#store.clock.now()))) {
            const renamed = await this.#tunnel(destination, "rename", {
                method: "POST",
                body: JSON.stringify({ name }),
            });
            await requireAnswer(renamed, `rename machine ${machineId}`);

            return;
        }

        // close the tunnel of a revoked or keyless machine, withdrawing its endpoint
        const reason = name === undefined ? "machine-revoked" : "key-revoked";
        const closing = await this.#tunnel(destination, "close", { method: "POST" });
        const answer = await requireAnswer(closing, `close machine ${machineId}`);
        if (!Closing.parse(JSON.parse(answer)).closed) {
            return;
        }
        await this.withdraw(machineId);

        // record the close as the relay in the machine's account history
        const now = this.#store.clock.now();
        const caller = Caller.of(this.#store.principal, now);
        await this.#record(caller, copied.accountId, machineId, (recorder, transaction) =>
            recorder.record(transaction, tunnelClose, {
                target: { type: "machine", id: machineId },
                details: { reason },
                outcome: { kind: "success" },
            }),
        );
    }

    /** Read a copied machine's destination and kind while it stands. */
    async #machine(machineId: Identifier<"machine">): Promise<
        | {
              readonly destination: MachineDestination;
              readonly accountId: string;
              readonly kind: Machine["kind"];
              readonly name: string | undefined;
          }
        | undefined
    > {
        // read the machine with its account's handle and residency
        const [copied] = await this.#store.database
            .select({
                machine: machine.table.name,
                accountId: machine.table.scope,
                kind: machine.table.kind,
                revokedAt: machine.table.revokedAt,
                handle: account.table.handle,
                residencyId: account.table.residencyId,
            })
            .from(machine.table)
            .innerJoin(account.table, eq(account.table.id, machine.table.scope))
            .where(eq(machine.table.id, machineId));
        if (copied === undefined) {
            return undefined;
        }

        // name the machine only while it stands
        return {
            destination: { machineId, residencyId: copied.residencyId },
            accountId: copied.accountId,
            kind: copied.kind,
            name:
                copied.revokedAt === null
                    ? MachineAddress.format(copied, this.domains.machine)
                    : undefined,
        };
    }

    /** List the standing machines of an account. */
    async machines(accountId: Identifier<"account">): Promise<readonly Identifier<"machine">[]> {
        const rows = await this.#store.database
            .select({ id: machine.table.id })
            .from(machine.table)
            .where(and(eq(machine.table.scope, accountId), isNull(machine.table.revokedAt)));

        return rows.map((row) => row.id);
    }

    /** Admit a machine opening its tunnel with the token it offers as a WebSocket subprotocol. */
    async admit(request: Request, now: number): Promise<Admission> {
        // require a machine's token
        const token = TunnelProtocol.token(request.headers.get("sec-websocket-protocol"));
        if (token === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer subprotocol" });
        }
        const { machineId, caller } = await Tunnel.admit(this.tokens, token, now);

        // require its key and the machine to stand
        await Machine.requireAuthenticating(this.#store.database, caller, now);
        const copied = await this.#machine(machineId);
        if (copied?.name === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: `machine ${machineId} is revoked` });
        }
        const { destination, accountId, name } = copied;

        // record the opening as the machine in its account's history
        await this.#record(caller, accountId, machineId, (recorder, transaction) =>
            recorder.record(transaction, tunnelOpen, {
                target: { type: "machine", id: machineId },
                details: { name },
                outcome: { kind: "success" },
            }),
        );

        // publish the relay's origin as where the machine answers, under a new publication kept on its tunnel
        const publication = crypto.randomUUID();
        const published = { scope: accountId, id: machineId, url: this.#origin.href, publication };
        await this.#call(Call.wire(machine.call("publish", published)));
        await this.#keepTunnel(machineId, name, publication);

        return {
            ...destination,
            lapsesAt: Caller.requireCredential(caller).expiresAt,
            name,
            publication,
        };
    }

    /** Withdraw the endpoint of a machine whose tunnel to the relay closed. */
    async withdraw(machineId: Identifier<"machine">): Promise<void> {
        // read the publication of the machine's tunnel, none once closed
        const id = Tunnel.id(machineId);
        const { database } = this.#store;
        const [kept] = await database
            .select({ publication: tunnel.table.publication })
            .from(tunnel.table)
            .where(eq(tunnel.table.id, id));
        if (kept?.publication === undefined || kept.publication === null) {
            return;
        }

        // withdraw the endpoint and clear the publication
        const accountId = await this.#account(machineId);
        const withdrawn = { scope: accountId, id: machineId, publication: kept.publication };
        await this.#call(Call.wire(machine.call("unpublish", withdrawn)));
        await database
            .update(tunnel.table)
            .set({ publication: null, updatedAt: this.#store.clock.now() })
            .where(eq(tunnel.table.id, id));
    }

    /** Keep a machine's tunnel row with the name the relay routes to it. */
    async #keepTunnel(
        machineId: Identifier<"machine">,
        name: string,
        publication: string,
    ): Promise<void> {
        const id = Tunnel.id(machineId);
        const now = this.#store.clock.now();
        await Transaction.run(this.#store.database, async (transaction) => {
            // update the tunnel the machine keeps
            const kept = await transaction.row(tunnel.table, { id });
            if (kept !== null) {
                await transaction.write({
                    table: tunnel.table,
                    before: kept,
                    after: {
                        ...kept,
                        name,
                        publication,
                        updatedAt: now,
                    },
                });

                return;
            }

            // create it, recording the tunnel as a partition in the universe
            const created = { id, name, publication, createdAt: now, updatedAt: now };
            await transaction.write({
                table: tunnel.table,
                before: null,
                after: tunnel.table[TABLE].complete(created),
            });
            const self = this.#store.principal;
            const authorization = new Authorization(this.#store.authorizer, transaction, () =>
                AccessContext.of(Caller.of(self, now), now),
            );
            await authorization.create(tunnel.reference(Scope.universe.id, id), {});
        });
    }

    /** Report whether a machine keeps a standing key, unrevoked, unsuspended and unexpired. */
    async #isKeyed(machineId: Identifier<"machine">, now: number): Promise<boolean> {
        const [kept] = await this.#store.database
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

        return kept !== undefined;
    }

    /** Record a tunnel's event as a caller in its machine's account history. */
    async #record(
        caller: Caller,
        accountId: string,
        machineId: Identifier<"machine">,
        record: (
            recorder: AuditRecorder<DatabaseConnection>,
            transaction: DatabaseConnection,
        ) => Promise<unknown>,
    ): Promise<void> {
        const recorder = AuditRecorder.from(caller, this.journal, {
            package: RELAY_PACKAGE,
            service: relayService.name,
            scope: accountId,
            machineId,
        });
        await this.journal.database.transaction((transaction) => record(recorder, transaction));
    }

    /** Forward a request for a name to its machine. */
    async fetch(request: Request): Promise<Response> {
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
        const host = FORWARDED_HOST.exec(forwarded)?.[1];
        const proto = FORWARDED_PROTO.exec(forwarded)?.[1];
        if (host === undefined || proto === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: "the forwarded request names no host and protocol",
            });
        }

        // ask for the name as the client did
        const restored = new Request(`${proto}://${host}${url.pathname}${url.search}`, request);
        restored.headers.delete("forwarded");

        return restored;
    }

    /** Forward a request for a name to its destination, kept or resolved. */
    async #forward(request: Request, isForwarded: boolean): Promise<Response> {
        // find the name's kept or resolved destination
        const name = new URL(request.url).hostname;
        const now = this.#store.clock.now();
        const kept = this.#destinations?.get(name, now);
        const destination = kept ?? (await this.#resolve(name));
        if (kept === undefined) {
            this.#destinations?.set(name, destination, now);
        }

        // refuse a forwarded request for a machine tunnelled elsewhere, which its relay resolves again
        if (isForwarded && !("machineId" in destination)) {
            this.#destinations?.delete(name);
            throw new ServiceError("MISDIRECTED_REQUEST", {
                message: `${name} is tunnelled to no machine at this relay`,
            });
        }

        // forward and drop a kept destination the machine answers misdirected
        const response = await this.#send(destination, request);
        if (kept === undefined || response.status !== ServiceError.status("MISDIRECTED_REQUEST")) {
            return response;
        }
        this.#destinations?.delete(name);

        // resolve once more for a bodiless request, dropping the misdirected answer, and ask one with a body to retry (RFC 9110)
        if (request.body !== null) {
            return response;
        }
        await response.body?.cancel();

        return this.#forward(request, isForwarded);
    }

    /** Forward a request to a machine through its tunnel (RFC 7239). */
    async #send(destination: Destination, request: Request): Promise<Response> {
        // call a machine through its tunnel
        if ("machineId" in destination) {
            const headers = new Headers(request.headers);
            headers.set(FORWARDED_URL_HEADER, request.url);

            return this.#tunnel(destination, "forward", {
                method: request.method,
                headers,
                body: request.body,
                ...(request.body === null ? {} : { duplex: "half" }),
            });
        }

        // keep the path and query and streamed body under the endpoint's origin
        const url = new URL(request.url);
        const target = new URL(`${url.pathname}${url.search}`, destination.endpoint);
        const forwarded = new Request(target.href, request);
        forwarded.headers.set("forwarded", `host="${url.host}";proto=${url.protocol.slice(0, -1)}`);

        return this.#fetch(forwarded);
    }

    /** Find where a name's requests go through the copies of the directory and the machines. */
    async #resolve(name: string): Promise<Destination> {
        const origin = InstallationOrigin.parse(name, this.domains.space);
        const computer = MachineAddress.parse(name, this.domains.machine);

        // route a space's name, and its branches', to the machine its placement names
        if (origin !== undefined) {
            return this.#locateSpace(name, SpaceAddress.of(origin));
        }
        // route a machine's name to its tunnel
        else if (computer !== undefined) {
            return this.#locateMachine(name, computer.handle, computer.machine);
        }
        // resolve nothing else
        else {
            throw new ServiceError("NOT_FOUND", { message: `${name} resolves to nothing` });
        }
    }

    /** Find the machine serving a space by its address through the directory's copy. */
    async #locateSpace(name: string, address: SpaceAddress): Promise<Destination> {
        // find the space's placement through the copied claims
        const resolver = new HandleResolver(this.#directory, this.#claims);
        const named = await resolver.find(space, address);
        const placed = named === undefined ? undefined : await resolver.directory.locate(named.id);
        if (placed === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${name} resolves to nothing` });
        }

        return this.#destination(schema.identifier("machine").parse(placed.machine));
    }

    /** Run a call of the account service as the relay installation. */
    async #call(call: sync.Call): Promise<void> {
        const now = this.#store.clock.now();
        await this.#publisher.receive({ id: RequestId.create(now), calls: [call] });
    }

    /** Read the account a copied machine belongs to. */
    async #account(machineId: Identifier<"machine">): Promise<Identifier<"account">> {
        const [copied] = await this.#store.database
            .select({ scope: machine.table.scope })
            .from(machine.table)
            .where(eq(machine.table.id, machineId));

        return schema
            .identifier("account")
            .parse(present(copied, `the copy of ${machineId}`).scope);
    }

    /** Find a standing machine of any residency by its name within its account. */
    async #locateMachine(name: string, handle: string, machineName: string): Promise<Destination> {
        // find the account by its handle, and its machine by name
        const accountId = await new HandleResolver(this.#directory, this.#claims).account(handle);
        const machineId =
            accountId === undefined
                ? undefined
                : await Machine.find(this.#store.database, accountId, machineName);
        if (machineId === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${name} resolves to nothing` });
        }

        return this.#destination(machineId);
    }

    /** Find where a machine's requests go. */
    async #destination(machineId: Identifier<"machine">): Promise<Destination> {
        // route to a person's computer of the relay's residency through its tunnel
        const copied = await this.#machine(machineId);
        if (copied?.kind === "personal") {
            return copied.destination;
        }

        // route to a cloud machine, or to the relay another residency's computer is tunnelled to
        const url = await this.#directory.url(machineId);
        if (url === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `machine ${machineId} is not connected`,
            });
        }

        return { endpoint: url };
    }
}

/** Keep the reads of a relay's copy of the directory current while the relay runs. */
export class DirectoryController implements Controller {
    /** The controller's name in reports. */
    readonly name = "directory";
    /** Follow the copy's log until the relay stops. */
    readonly mode = "follow";
    /** The relay reading the copy. */
    readonly #relay: Relay;

    /** Follow the copy a relay reads. */
    constructor(relay: Relay) {
        this.#relay = relay;
    }

    /** List the one copy. */
    async list(): Promise<readonly string[]> {
        return ["directory"];
    }

    /** Follow the copy's log until stopped. */
    async reconcile(_key: string, reconciliation: Reconciliation): Promise<undefined> {
        await this.#relay.follow(reconciliation.signal);

        return undefined;
    }
}

/** Tell each connected machine its name once its own or its account's copy changes. */
export class NameController implements Controller {
    /** The controller's name in reports. */
    readonly name = "name";
    /** The copied machines, their keys and the accounts names derive from. */
    readonly watches: readonly Table[] = [machine.table, key.table, account.table];
    /** The relay keeping the tunnels. */
    readonly #relay: Relay;

    /** Tell the machines connected to a relay their names. */
    constructor(relay: Relay) {
        this.#relay = relay;
    }

    /** List the machine whose name or revocation changed. */
    async keys(change: Change): Promise<readonly string[]> {
        // select a machine that was renamed or revoked or created or deleted
        if (Change.of(change, machine.table)) {
            const before = Change.before(change);
            const after = Change.after(change);
            const isNamed = before?.name !== after?.name || before?.revokedAt !== after?.revokedAt;

            return isNamed ? [Change.image(change).id] : [];
        }
        // select the machine of a key that was revoked, suspended, restored or removed
        else if (Change.of(change, key.table)) {
            const before = Change.before(change);
            const after = Change.after(change);
            const image = Change.image(change);
            const isStanding =
                before?.revokedAt !== after?.revokedAt ||
                before?.suspendedAt !== after?.suspendedAt;

            return image.parentType === machine.name && image.parentId !== null && isStanding
                ? [image.parentId]
                : [];
        }
        // select the machines of an account whose handle changed
        else if (Change.of(change, account.table)) {
            const isRenamed = Change.before(change)?.handle !== Change.after(change)?.handle;

            return isRenamed ? this.#relay.machines(Change.image(change).id) : [];
        }
        // select nothing for other tables
        else {
            return [];
        }
    }

    /** List no machine at start, since each tunnel takes its name as it opens. */
    async list(): Promise<readonly string[]> {
        return [];
    }

    /** Tell a connected machine its name, or close its revoked machine's tunnel. */
    async reconcile(machineId: string): Promise<undefined> {
        await this.#relay.reconcile(schema.identifier("machine").parse(machineId));

        return undefined;
    }
}

/** Read a call's answer to the end, refusing a failed one. */
async function requireAnswer(response: Response, action: string): Promise<string> {
    const text = await response.text();
    if (!response.ok) {
        throw new ServiceError("BAD_GATEWAY", {
            message: `${action} failed with ${response.status}: ${text}`,
        });
    }

    return text;
}

import type { Fetch } from "@destack/service";
import type { DirectoryClient } from "@destack/account/client";
import { Resolver } from "@destack/account/directory";
import { AuditRecorder, Journal } from "@destack/audit/server";
import { and, Change, eq, gt, isNull, type DatabaseConnection, type Table } from "@destack/db";
import { zoneTable } from "@destack/directory";
import {
    DOMAINS,
    type Domains,
    MachineAddress,
    InstallationOrigin,
} from "@destack/account/address";
import { account, key, Machine, machine } from "@destack/account/object";
import { schema, type Identifier } from "@destack/schema";
import type { TokenVerifier } from "@destack/service/authentication";
import type { AuditCaller } from "@destack/audit";
import type { CallKey } from "@destack/service/request";
import type {} from "@destack/package/import-meta";
import type { Controller } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { refusal, reportError } from "@destack/service/server";
import { space, SpaceCell } from "@destack/space/object";
import { tunnelClose, tunnelOpen } from "../audit/index.ts";
import { relayService } from "../service/index.ts";
import { TunnelProtocol } from "../session/index.ts";
import type { DestinationCache } from "./destination.ts";
import {
    type Admission,
    type MachineDestination,
    Tunnel,
    TUNNEL_PATH,
    type TunnelHost,
} from "./tunnel.ts";

/** The relay's package, the audience of the tokens machines open their tunnels with. */
export const RELAY_PACKAGE = import.meta.destack.package;

/** The lifetime of the tokens machines renew their tunnels with: ten minutes, so a tunnel renews rarely. */
export const RELAY_TOKEN_LIFETIME_MILLISECONDS = 10 * 60_000;

/** What a relay routes by, and where. */
export interface RelayOptions {
    /** The relay's own origin, where machines open their tunnels. */
    readonly origin: string;
    /** The relay's database keeping copies of the accounts, machines, machine keys and zones names resolve with, or its opening on first use, as an edge Worker opens it only on a missed name. */
    readonly database: DatabaseConnection | (() => Promise<DatabaseConnection>);
    /** The universe's directory, which names' claims and cells' endpoints resolve through. */
    readonly directory: DirectoryClient;
    /** Verify the universe's tokens machines open tunnels with, for the relay's package. */
    readonly tokens: TokenVerifier;
    /** The key the relay's journal fingerprints sensitive values under, which records tunnels opening and closing. */
    readonly callKey: CallKey;
    /** Where the machines keep their tunnels. */
    readonly tunnels: TunnelHost;
    /** Report a failure the relay answers to no one. */
    readonly report: (error: unknown) => void;
    /** The domains names resolve under. */
    readonly domains?: Domains;
    /** Reach regions at their published endpoints. */
    readonly fetch?: Fetch;
    /** The destinations an edge Worker's isolate keeps across requests, absent where the copies are local. */
    readonly destinations?: DestinationCache;
}

/** Where the relay forwards a name's requests: a machine's tunnel, or a region's published endpoint. */
export type Destination = MachineDestination | { readonly endpoint: string };

/** The edge of Destack's names: requests for a space's or a machine's name go to the machine through its tunnel, or to the region serving it. */
export class Relay {
    /** Verify the universe's tokens machines open and renew their tunnels with. */
    readonly tokens: TokenVerifier;
    /** The domains names resolve under. */
    readonly domains: Domains;
    /** Report a failure the relay answers to no one. */
    readonly report: (error: unknown) => void;
    /** The relay's database with the copies, opened on first use. */
    readonly #database: () => Promise<DatabaseConnection>;
    /** The key the journal fingerprints under. */
    readonly #callKey: CallKey;
    /** The resolver of handles from the copies and of names through the directory. */
    readonly #resolver: Resolver;
    /** Where the machines keep their tunnels. */
    readonly #tunnels: TunnelHost;
    /** The relay's own origin. */
    readonly #origin: URL;
    /** Call regions. */
    readonly #fetch: Fetch;
    /** The destinations kept across requests, absent where the copies are local. */
    readonly #destinations: DestinationCache | undefined;

    /** Route as the options describe. */
    constructor(options: RelayOptions) {
        // open the database once on first use, and resolve handles from its copies and names through the directory
        const { database } = options;
        let opened: Promise<DatabaseConnection> | undefined;
        this.#database = () => {
            opened ??= typeof database === "function" ? database() : Promise.resolve(database);

            return opened;
        };
        this.#resolver = new Resolver(options.directory, async (handle) =>
            Resolver.account(await this.#database(), handle),
        );

        // keep the token verifier, the journal's key and the calls to tunnels and regions
        this.tokens = options.tokens;
        this.#callKey = options.callKey;
        this.domains = options.domains ?? DOMAINS;
        this.report = options.report;
        this.#tunnels = options.tunnels;
        this.#origin = new URL(options.origin);
        this.#fetch = options.fetch ?? globalThis.fetch;
        this.#destinations = options.destinations;
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

    /** Bring a machine's tunnel in line with its copy: tell it its name, or close it once the machine or its last standing key is revoked, recording the close. */
    async reconcile(machineId: Identifier<"machine">): Promise<void> {
        // leave a machine the copies no longer keep
        const copied = await this.#machine(machineId);
        if (copied === undefined) {
            return;
        }

        // tell a standing machine with a standing key its name
        const { destination, name } = copied;
        if (name !== undefined && (await this.#isKeyed(machineId))) {
            await this.#tunnels.rename(destination, name);

            return;
        }

        // close the tunnel of a revoked or keyless machine, recording the close in its account's history
        const reason = name === undefined ? "machine-revoked" : "key-revoked";
        if (await this.#tunnels.close(destination)) {
            await this.#record(
                machineId,
                { type: "system", name: "relay" },
                (recorder, transaction) =>
                    recorder.record(transaction, tunnelClose, {
                        target: { type: "machine", id: machineId },
                        details: { reason },
                        outcome: { kind: "success" },
                    }),
            );
        }
    }

    /** Read a copied machine's destination and the name the relay routes to it below the machine domain while it stands, absent for a machine the copies do not keep. */
    async #machine(
        machineId: Identifier<"machine">,
    ): Promise<
        { readonly destination: MachineDestination; readonly name: string | undefined } | undefined
    > {
        // read the machine with its account's handle and residency
        const database = await this.#database();
        const [copied] = await database
            .select({
                machine: machine.table.name,
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
            name:
                copied.revokedAt === null
                    ? MachineAddress.format(copied, this.domains.machine)
                    : undefined,
        };
    }

    /** List the standing machines of an account. */
    async machines(accountId: Identifier<"account">): Promise<readonly Identifier<"machine">[]> {
        const database = await this.#database();
        const rows = await database
            .select({ id: machine.table.id })
            .from(machine.table)
            .where(and(eq(machine.table.scope, accountId), isNull(machine.table.revokedAt)));

        return rows.map((row) => row.id);
    }

    /** Admit a machine opening its tunnel with the token it offers as a WebSocket subprotocol, while its key and the machine stand. */
    async open(request: Request, now = Date.now()): Promise<Admission> {
        // require a machine's token
        const token = TunnelProtocol.token(request.headers.get("sec-websocket-protocol"));
        if (token === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer subprotocol" });
        }
        const { machineId, caller } = await Tunnel.admit(this.tokens, token, now);

        // require its key and the machine to stand
        await Machine.requireAuthenticating(await this.#database(), caller, now);
        const copied = await this.#machine(machineId);
        if (copied?.name === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: `machine ${machineId} is revoked` });
        }
        const { destination, name } = copied;

        // record the opening as the machine in its account's history
        const journal = await this.#journal();
        const recorder = AuditRecorder.from(caller, journal, {
            package: RELAY_PACKAGE,
            service: relayService.name,
            scope: caller.claims.subject.scope,
            machineId,
        });
        await journal.database.transaction((transaction) =>
            recorder.record(transaction, tunnelOpen, {
                target: { type: "machine", id: machineId },
                details: { name },
                outcome: { kind: "success" },
            }),
        );

        return { ...destination, lapsesAt: caller.lapsesAt, name };
    }

    /** Report whether a machine keeps a standing key, unrevoked, unsuspended and unexpired. */
    async #isKeyed(machineId: Identifier<"machine">, now = Date.now()): Promise<boolean> {
        const database = await this.#database();
        const [kept] = await database
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

    /** Record a tunnel's event as a caller in its machine's account history, nothing for a machine its copies no longer keep. */
    async #record(
        machineId: Identifier<"machine">,
        caller: AuditCaller,
        record: (
            recorder: AuditRecorder<DatabaseConnection>,
            transaction: DatabaseConnection,
        ) => Promise<unknown>,
    ): Promise<void> {
        // find the machine's account
        const journal = await this.#journal();
        const [kept] = await journal.database
            .select({ scope: machine.table.scope })
            .from(machine.table)
            .where(eq(machine.table.id, machineId));
        if (kept === undefined) {
            return;
        }

        // record under the caller in one transaction
        const recorder = new AuditRecorder(
            {
                caller,
                package: RELAY_PACKAGE,
                service: relayService.name,
                scope: kept.scope,
                machineId,
            },
            journal,
        );
        await journal.database.transaction((transaction) => record(recorder, transaction));
    }

    /** Open the relay's journal over its database on first use. */
    async #journal(): Promise<Journal> {
        return new Journal(await this.#database(), this.#callKey);
    }

    /** Forward a request for a name to its cell, answering a failure with its status and message. */
    async fetch(request: Request): Promise<Response> {
        try {
            return await this.#forward(request);
        } catch (error) {
            return refusal(error);
        }
    }

    /** Forward a request for a name to its destination, kept or resolved, and resolve a kept one again once its cell answers it misdirected. */
    async #forward(request: Request): Promise<Response> {
        // find the name's kept or resolved destination
        const name = new URL(request.url).hostname;
        const kept = this.#destinations?.get(name);
        const destination = kept ?? (await this.#resolve(name));
        if (kept === undefined) {
            this.#destinations?.set(name, destination);
        }

        // forward and drop a kept destination the cell answers misdirected
        const response = await this.#send(destination, request);
        if (kept === undefined || response.status !== ServiceError.status("MISDIRECTED_REQUEST")) {
            return response;
        }
        this.#destinations?.delete(name);

        // resolve once more for a bodiless request and ask one with a body to retry (RFC 9110)
        return request.body === null ? this.#forward(request) : response;
    }

    /** Forward a request to a machine through its tunnel, or to a region at its endpoint naming the host asked for (RFC 7239). */
    async #send(destination: Destination, request: Request): Promise<Response> {
        // reach a machine through its tunnel
        if ("machineId" in destination) {
            return this.#tunnels.fetch(destination, request);
        }

        // keep the path and query and streamed body under the endpoint's origin
        const url = new URL(request.url);
        const target = new URL(`${url.pathname}${url.search}`, destination.endpoint);
        const forwarded = new Request(target.href, request);
        forwarded.headers.set("forwarded", `host="${url.host}";proto=${url.protocol.slice(0, -1)}`);

        return this.#fetch(forwarded);
    }

    /** Find where a name's requests go, refusing a name that resolves to nothing. */
    async #resolve(name: string): Promise<Destination> {
        // find the name's cell
        const cell = await this.#cell(name);
        if (cell === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${name} resolves to nothing` });
        } else if ("machineId" in cell) {
            return await this.#destination(cell.machineId, name);
        }

        // read a region's published endpoint
        const published = await this.#resolver.directory.cell(cell.regionId);
        if (published === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `region ${cell.regionId} published no endpoint`,
            });
        }

        return { endpoint: published.endpoint };
    }

    /** Read where a machine serving a name keeps its tunnel, refusing a machine the copies do not keep. */
    async #destination(
        machineId: Identifier<"machine">,
        name: string,
    ): Promise<MachineDestination> {
        const copied = await this.#machine(machineId);
        if (copied === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${name} resolves to nothing` });
        }

        return copied.destination;
    }

    /** Find the machine or region serving a name. */
    async #cell(name: string): Promise<SpaceCell | undefined> {
        const origin = InstallationOrigin.parse(name, this.domains.space);
        const computer = MachineAddress.parse(name, this.domains.machine);

        // find the cell of a space's copied zone, which also serves its branches
        if (origin !== undefined) {
            const named = await this.#resolver.find(space, `${origin.space}.${origin.handle}`);
            const [placed] =
                named === undefined
                    ? []
                    : await (
                          await this.#database()
                      )
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
                    : await Machine.find(await this.#database(), accountId, computer.machine);

            return machineId === undefined ? undefined : { machineId };
        }
        // resolve nothing else
        else {
            return undefined;
        }
    }
}

/** Tell each connected machine its name once its own or its account's copy changes, as a rename or a new handle does, and close the tunnel of a revoked machine or one whose last key was revoked. */
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

    /** List the machine whose name or revocation changed, or the machines of an account whose handle changed, leaving other changes such as a machine's reports. */
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

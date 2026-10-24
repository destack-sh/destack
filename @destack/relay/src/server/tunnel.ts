import { Bearer } from "@destack/access/token";
import type { Environment } from "@destack/environment";
import { refusal } from "@destack/object";
import type { Hibernation, Socket, SocketHandler } from "@destack/object/workload";
import type {} from "@destack/package/import-meta";
import { Identifier, present, schema, Uuid } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import { RelayError } from "../error/index.ts";
import relayPackage from "../package.ts";
import {
    MAX_STREAMS,
    PING_MESSAGE,
    PONG_MESSAGE,
    type Renewal,
    Session,
    type Stream,
    TUNNEL_PROTOCOL,
} from "../session/index.ts";
import type { TunnelToken } from "./token.ts";

/** The path below the relay package's mount of the calls between the root and each tunnel. */
export const TUNNELS_PATH = relayPackage.adapters.tunnels.path;

/** The header with a machine's accepted opening handshake. */
export const HANDSHAKE_HEADER = "Destack-Handshake";

/** The path machines open and renew their tunnels at. */
export const TUNNEL_PATH = "/tunnel";

/** How long a machine may take to answer a request's head: 100 s, Cloudflare's read timeout. */
export const ANSWER_TIMEOUT_MILLISECONDS = 100_000;

/** The prefix of a machine's tunnel partition, beside the machine's identifier's UUID. */
const PARTITION_PREFIX = "tunnel-";

/** The errors the relay answers for each reason a machine no longer stands. */
export const STANDING_ERRORS = {
    "machine-revoked": ["MACHINE_REVOKED", "is revoked"],
    "key-revoked": ["MACHINE_UNKEYED", "has no key"],
    "machine-unknown": ["MACHINE_UNKNOWN", "is not in the directory"],
} as const;

/** The answers that end a socket's standing: unauthorized and forbidden (RFC 9110 15.5). */
const REFUSED_STATUSES: ReadonlySet<number> = new Set([401, 403]);

/** A machine's accepted opening handshake (RFC 6455 section 4): its token's lapse, key and name. */
export const Handshake = schema.object({
    /** The machine. */
    machineId: schema.identifier("machine"),
    /** When its token lapses, in UTC epoch milliseconds. */
    lapsesAt: schema.number(),
    /** The name the relay routes to the machine, such as `laptop--florian.destack.computer`. */
    name: schema.string().min(1),
    /** The account the machine is enrolled in, which its endpoint lives in. */
    account: schema.identifier("account"),
    /** The universe key that signed the machine's token. */
    kid: schema.string().min(1),
});
/** A machine's accepted opening handshake. */
export type Handshake = schema.Infer<typeof Handshake>;

/** The value stored with a hibernated tunnel socket: its machine, its token's lapse and key. */
export const TunnelAttachment = schema.object({
    /** The machine the socket connects. */
    machineId: schema.identifier("machine"),
    /** When the socket's token lapses, in UTC epoch milliseconds. */
    lapsesAt: schema.number(),
    /** The universe key that signed the socket's token. */
    kid: schema.string().min(1),
});
/** The value stored with a hibernated tunnel socket. */
export type TunnelAttachment = schema.Infer<typeof TunnelAttachment>;

/** The calls the relay's root makes to a machine's tunnel in its partition. */
export type TunnelCall = "open" | "forward" | "rename" | "close";

/** Why a tunnel's socket ends: a revoke, a lapsed token, its endpoint's move, or its close. */
export const TunnelCloseReason = schema.enum([
    "machine-revoked",
    "key-revoked",
    "machine-unknown",
    "lapsed",
    "moved",
    "closed",
]);
/** Why a tunnel's socket ends. */
export type TunnelCloseReason = schema.Infer<typeof TunnelCloseReason>;

/** Why a machine does not stand here: its revoke, no standing key, or no directory entry. */
export const TunnelStandingReason = schema.enum([
    "machine-revoked",
    "key-revoked",
    "machine-unknown",
]);

/** Whether a machine stands: the name the relay routes to it, or why its tunnel ends. */
export const TunnelStanding = schema.union([
    schema.object({
        /** The name the relay routes to the standing machine. */
        name: schema.string().min(1),
        /** The account the machine is enrolled in, which its endpoint lives in. */
        account: schema.identifier("account"),
    }),
    schema.object({
        /** Why the tunnel of a machine that no longer stands ends. */
        reason: TunnelStandingReason,
    }),
]);
/** Whether a machine stands. */
export type TunnelStanding = schema.Infer<typeof TunnelStanding>;

/** One socket of a machine's tunnel: a hibernated socket's session until its token lapses. */
interface TunnelSocket {
    /** The hibernated socket's identifier. */
    readonly id: string;
    /** The session over the socket. */
    readonly session: Session;
    /** When the socket's token lapses, in UTC epoch milliseconds, and the key that signed it. */
    attachment: TunnelAttachment;
    /** Whether the socket stands as of this instance: accepted here, or verified since it woke. */
    isVerified: boolean;
}

/** What a machine's tunnel runs with in its partition. */
export interface TunnelOptions {
    /** The host of the partition's hibernated sockets. */
    readonly hibernation: Pick<Hibernation, "list" | "socket" | "accept" | "attach">;
    /** The relay's tunnel URL the machine dials, which its renames come from. */
    readonly url: string;
    /** The universe's tunnel tokens and keys, which machines renew their tunnels with. */
    readonly tokens: Pick<TunnelToken, "verify" | "isStanding">;
    /** Wait until the partition follows the universe's identities, whose keys verify the tokens. */
    readonly identities: () => Promise<void>;
    /** Read from the relay's root whether the machine stands, and its name. */
    readonly standing: () => Promise<TunnelStanding>;
    /** Tell the lapses' controller a socket opened or its lapse moved. */
    readonly changed: () => void;
    /** Publish the machine's tunnel here at the universe, lapsing with its token. */
    readonly publish: (account: Identifier<"account">, lapsesAt: number) => Promise<void>;
    /** The clock, randomness and telemetry the tunnel's lapses, socket ids and records run on. */
    readonly environment: Pick<Environment, "clock" | "random" | "telemetry">;
}

/** A machine's tunnel to the relay in its partition: its hibernated sockets and their sessions. */
export class Tunnel implements SocketHandler {
    /** The machine the tunnel leads to. */
    readonly machineId: Identifier<"machine">;
    /** What the tunnel runs with. */
    readonly #options: TunnelOptions;
    /** The tunnel's log records and captures. */
    readonly #instruments: TelemetryScope;
    /** The sockets running a session, oldest first. */
    readonly #sockets: TunnelSocket[] = [];
    /** The sockets accepted here that the host has not delivered an event of yet, by identifier. */
    readonly #accepted = new Map<string, TunnelAttachment>();
    /** The sessions over the sockets hibernated before this instance started, once listed. */
    #listing: Promise<void> | undefined;
    /** The name the root last told the tunnel, absent since the instance started. */
    #name: string | undefined;
    /** The account the machine is enrolled in, as its opening or its last renewal read it. */
    #account: Identifier<"account"> | undefined;

    /** Run a machine's tunnel in its partition. */
    constructor(machineId: Identifier<"machine">, options: TunnelOptions) {
        this.machineId = machineId;
        this.#options = options;
        this.#instruments = TelemetryScope.of(
            import.meta.destack.package,
            options.environment.telemetry,
        );
    }

    /** Derive the partition of a machine's tunnel. */
    static partition(machineId: Identifier<"machine">): string {
        return `${PARTITION_PREFIX}${Identifier.uuid(machineId)}`;
    }

    /** Read the machine a tunnel's partition leads to. */
    static machine(partition: string): Identifier<"machine"> {
        return schema
            .identifier("machine")
            .parse(`machine-${partition.slice(PARTITION_PREFIX.length)}`);
    }

    /** Accept the machine's WebSocket after its handshake, keeping its lapse and key beside it. */
    accept(handshake: Handshake): Response {
        // remember the accepted socket until the host delivers its first event
        const id = Uuid.v4(this.#options.environment.random);
        const attachment: TunnelAttachment = {
            machineId: this.machineId,
            lapsesAt: handshake.lapsesAt,
            kid: handshake.kid,
        };
        this.#name = handshake.name;
        this.#account = handshake.account;
        this.#accepted.set(id, attachment);

        return this.#options.hibernation.accept({
            id,
            attachment,
            protocol: TUNNEL_PROTOCOL,
            respond: { request: PING_MESSAGE, response: PONG_MESSAGE },
        });
    }

    /** Take a hibernated socket's message into its session. */
    async message(socket: Socket, message: Uint8Array<ArrayBuffer>): Promise<void> {
        await this.#list();
        this.#running(socket)?.session.receive(message);
    }

    /** Forget a hibernated socket once it closed. */
    async close(socket: Socket): Promise<void> {
        // forget a socket that never ran, and close a running one's session
        await this.#list();
        this.#accepted.delete(socket.id);
        const closed = this.#remove(socket.id);
        if (closed !== undefined) {
            closed.session.terminate();
            this.#closed("closed");
        }
    }

    /** Close every socket for a reason, telling the machine. */
    async closeAll(reason: TunnelCloseReason): Promise<void> {
        await this.#list();
        this.#closeAll(reason);
    }

    /** Close the sockets that no longer stand or lapsed, answering the wait for the next lapse. */
    async closeLapsed(): Promise<number | undefined> {
        // close the woken sockets of a machine or key that no longer stands
        await this.#list();
        await this.#verifyWoken();

        // close each lapsed socket, telling the machine
        const now = this.#options.environment.clock.now();
        for (const lapsed of this.#sockets.filter((each) => each.attachment.lapsesAt <= now)) {
            this.#remove(lapsed.id)?.session.close();
            this.#closed("lapsed");
        }

        // wait for the next lapse
        const lapses = this.#sockets.map((each) => each.attachment.lapsesAt);

        return lapses.length === 0 ? undefined : Math.min(...lapses) - now;
    }

    /** Forward a request through the newest verified socket whose token has not lapsed. */
    async fetch(request: Request): Promise<Response> {
        // verify the sockets woken since the partition slept, then require one with room for another request
        await this.#list();
        await this.#verifyWoken();
        const now = this.#options.environment.clock.now();
        const newest = this.#sockets.findLast((each) => each.attachment.lapsesAt > now);
        if (newest === undefined) {
            throw RelayError.of(
                "MACHINE_DISCONNECTED",
                `machine ${this.machineId} is disconnected`,
            );
        } else if (newest.session.size >= MAX_STREAMS) {
            throw RelayError.of(
                "STREAMS_EXHAUSTED",
                `machine ${this.machineId} already has ${MAX_STREAMS} open requests`,
            );
        }

        return this.#send(newest.session, request);
    }

    /** Take the machine's name from the root, asking the machine to renew once it changed. */
    async rename(name: string): Promise<void> {
        // leave a name the tunnel has, and a machine without a socket
        await this.#list();
        const isRenamed = this.#name !== name;
        this.#name = name;
        if (!isRenamed || this.#sockets.length === 0) {
            return;
        }

        // ask the connected machine to renew, to learn the name
        const response = await this.fetch(new Request(this.#options.url, { method: "PUT" }));
        if (!response.ok) {
            throw RelayError.of(
                "RENEWAL_REFUSED",
                `machine ${this.machineId} refused to renew: ${response.status}`,
            );
        }
    }

    /** Read the running socket of a host's event, running one accepted here on its first event. */
    #running(socket: Socket): TunnelSocket | undefined {
        // take a running socket
        const running = this.#sockets.find((each) => each.id === socket.id);
        const accepted = this.#accepted.get(socket.id);
        if (running !== undefined || accepted === undefined) {
            return running;
        }

        // run the socket the host connected, telling the lapses' controller and publishing the tunnel
        this.#accepted.delete(socket.id);
        const added = this.#attach(socket, accepted, true);
        this.#instruments.log.info("tunnel.open", { machine: this.machineId });
        this.#options.changed();
        this.#publish(accepted.lapsesAt);

        return added;
    }

    /** Run a session over a socket, taking the machine's renewals on the streams it opens. */
    #attach(socket: Socket, attachment: TunnelAttachment, isVerified: boolean): TunnelSocket {
        const added: TunnelSocket = {
            id: socket.id,
            attachment,
            isVerified,
            session: new Session(socket, "server", {
                accept: (stream) =>
                    void this.#renew(added, stream).catch((error: unknown) =>
                        this.#instruments.captureException(error),
                    ),
            }),
        };
        this.#sockets.push(added);

        return added;
    }

    /** Run a session over each socket hibernated before this instance started, once listed. */
    #list(): Promise<void> {
        // list once, and again after a failed listing
        this.#listing ??= this.#listHibernated().catch((error: unknown) => {
            this.#listing = undefined;
            throw error;
        });

        return this.#listing;
    }

    /** Run an unverified session over each socket hibernated before this instance started. */
    async #listHibernated(): Promise<void> {
        const listed = await this.#options.hibernation.list();
        const woken = listed
            .filter((socket) => !this.#accepted.has(socket.id))
            .filter((socket) => !this.#sockets.some((each) => each.id === socket.id))
            .map((socket) => ({ socket, attachment: TunnelAttachment.parse(socket.attachment) }))
            .toSorted((left, right) => left.attachment.lapsesAt - right.attachment.lapsesAt);
        for (const { socket, attachment } of woken) {
            this.#attach(socket, attachment, false);
        }
    }

    /** Close the woken sockets of a machine that no longer stands, or of a revoked key. */
    async #verifyWoken(): Promise<void> {
        // close every socket of a machine revoked or left keyless while the partition slept
        const woken = this.#sockets.filter((each) => !each.isVerified);
        if (woken.length === 0) {
            return;
        }
        const standing = await this.#options.standing();
        if ("reason" in standing) {
            this.#closeAll(standing.reason);

            return;
        }

        // close each woken socket whose token's key the universe revoked, keeping the others
        await this.#options.identities();
        for (const socket of woken) {
            // keep a socket whose key stands, and close one a concurrent close did not take meanwhile
            const isStanding = await this.#options.tokens.isStanding(socket.attachment.kid);
            const removed = isStanding ? undefined : this.#remove(socket.id);
            if (isStanding) {
                socket.isVerified = true;
            } else if (removed !== undefined) {
                removed.session.close();
                this.#closed("key-revoked");
            }
        }
    }

    /** Close every running socket for a reason, telling the machine. */
    #closeAll(reason: TunnelCloseReason): void {
        for (const closed of this.#sockets.splice(0)) {
            closed.session.close();
            this.#closed(reason);
        }
    }

    /** Take a socket out of the tunnel, absent when none has the identifier. */
    #remove(id: string): TunnelSocket | undefined {
        const index = this.#sockets.findIndex((each) => each.id === id);

        return index === -1 ? undefined : this.#sockets.splice(index, 1)[0];
    }

    /** Record a socket's close as an event of the machine's tunnel. */
    #closed(reason: TunnelCloseReason): void {
        this.#instruments.log.info("tunnel.close", { machine: this.machineId, reason });
    }

    /** Store a socket's new lapse and key with it at the host. */
    #relapse(socket: TunnelSocket, attachment: TunnelAttachment): void {
        // move the lapse here and at the host, telling the lapses' controller
        socket.attachment = attachment;
        this.#options.hibernation.attach(socket.id, attachment);
        this.#options.changed();
    }

    /** Send a request over a session. */
    async #send(session: Session, request: Request): Promise<Response> {
        // abort the request once the answer timeout passes on the tunnel's clock
        const expiring = new AbortController();
        const answered = new AbortController();
        void this.#options.environment.clock
            .sleep(ANSWER_TIMEOUT_MILLISECONDS, answered.signal)
            .then(
                () => expiring.abort(),
                () => undefined,
            );
        const signal = AbortSignal.any([request.signal, expiring.signal]);

        // forward it and tell a timeout from the caller's own abort
        try {
            return await session.fetch(new Request(request, { signal }));
        } catch (error) {
            if (expiring.signal.aborted) {
                throw RelayError.of(
                    "HEAD_TIMEOUT",
                    `machine ${this.machineId} sent no response head within ${ANSWER_TIMEOUT_MILLISECONDS} ms`,
                );
            }
            throw error;
        } finally {
            answered.abort();
        }
    }

    /** Renew a socket's token through a stream the machine opened. */
    async #renew(renewing: TunnelSocket, stream: Stream): Promise<void> {
        // refuse any request but a renewal through the tunnel path
        const request = stream.request();
        const isRenewal = request.method === "PUT" && new URL(request.url).pathname === TUNNEL_PATH;
        if (!isRenewal) {
            await stream.respond(
                refusal(
                    RelayError.of("RENEWAL_ONLY", "machines can only renew their own tunnel"),
                    this.#options.environment.telemetry,
                ),
            );

            return;
        }

        // verify the token and the machine's standing, then store the lapse and answer the name
        let response: Response;
        try {
            await this.#options.identities();
            const attachment = await this.#verifyRenewal(request);
            const name = await this.#requireName();
            this.#relapse(renewing, attachment);
            this.#publish(attachment.lapsesAt);
            response = Response.json({ name, lapsesAt: attachment.lapsesAt } satisfies Renewal);
        }
        // leave a renewal whose session closed meanwhile, else refuse it
        catch (error) {
            if (renewing.session.isClosed) {
                return;
            }
            response = refusal(error, this.#options.environment.telemetry);
        }

        // answer the renewal, then lapse a socket whose token or standing the relay refused
        try {
            await stream.respond(response);
        } finally {
            if (REFUSED_STATUSES.has(response.status)) {
                this.#relapse(renewing, {
                    ...renewing.attachment,
                    lapsesAt: this.#options.environment.clock.now(),
                });
            }
        }
    }

    /** Read the name of the machine, refusing a machine that no longer stands for its reason. */
    async #requireName(): Promise<string> {
        // refuse a machine that no longer stands
        const standing = await this.#options.standing();
        if ("reason" in standing) {
            const [reason, refused] = STANDING_ERRORS[standing.reason];
            throw RelayError.of(reason, `machine ${this.machineId} ${refused}`);
        }

        // keep the machine's account, which its endpoint lives in
        this.#account = standing.account;

        return standing.name;
    }

    /** Publish the machine's tunnel at the universe in the background, reporting a failure. */
    #publish(lapsesAt: number): void {
        const account = present(this.#account, `the account of machine ${this.machineId}`);
        this.#options.publish(account, lapsesAt).catch((error: unknown) => {
            this.#instruments.captureException(error);
        });
    }

    /** Verify the machine's own renewed token in a request's bearer header. */
    async #verifyRenewal(request: Request): Promise<TunnelAttachment> {
        const verified = await this.#options.tokens.verify(
            Bearer.require(request.headers),
            this.#options.environment.clock.now(),
        );
        if (verified.machineId !== this.machineId) {
            throw RelayError.of("TUNNEL_FOREIGN", "machines can only renew their own tunnels");
        }

        return verified;
    }
}

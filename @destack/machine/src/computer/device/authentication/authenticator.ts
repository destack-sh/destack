import type { Machine } from "../../../machine/index.ts";
import { Scope, type Subject } from "@destack/sync";
import { TOKEN_LIFETIME_MILLISECONDS, Bearer, type PartitionId } from "@destack/access/token";
import type { PackageId } from "@destack/package";
import type { ProcedureContext, RequestContext } from "@destack/object/server";
import { Proof } from "@destack/identity";
import {
    type Authentication,
    Caller,
    CONTAINED,
    type Credential,
    principal,
} from "@destack/access";
import { login, type NativeClient } from "@destack/account/native";
import { and, eq, type DatabaseConnection } from "@destack/db";
import { Digest, schema } from "@destack/schema";
import { space } from "@destack/account/object";
import {
    type Clock,
    type Crypto,
    type Environment,
    Forwarded,
    HostAddress,
} from "@destack/environment";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../../../error/index.ts";

/** The refusal of a universe token bound to no space. */
const UNBOUND_REFUSAL = "universe tokens reach this machine bound to a partition";

/** The parts of a signed token: header, claims and signature (RFC 7519 7.2). */
const JWT_PARTS = 3;

/** The random bytes of the owner's credential, 256 bits. */
const OWNER_TOKEN_BYTES = 32;

/** The owner's credential, the proofs of people signed in here, and other machines' tokens. */
export class Authenticator {
    /** The owner's credential, published in the endpoint file. */
    readonly token: string;
    /** The native device's credential, which verifies a signed-in person's own client. */
    readonly #credential: Pick<NativeClient, "verify">;
    /** The native device whose logins name the people signed in here. */
    readonly #device: { readonly id: string; readonly database: DatabaseConnection };
    /** This machine as a principal. */
    readonly subject: Subject;
    /** This machine's identifier. */
    readonly machineId: string;
    /** The machine verifying callers on other machines and in other spaces. */
    readonly #machine: Pick<Machine, "verify" | "requireStanding" | "space">;
    /** The clock the credentials are verified at. */
    readonly #clock: Clock;
    /** The hashing the owner's credential is compared through. */
    readonly #crypto: Pick<Crypto, "digest">;

    /** Authenticate the owner as this machine, and signed-in people by their clients' proofs. */
    constructor(
        machineId: string,
        device: {
            readonly id: string;
            readonly database: DatabaseConnection;
            readonly credential: Pick<NativeClient, "verify">;
        },
        machine: Pick<Machine, "verify" | "requireStanding" | "space">,
        environment: Pick<Environment, "clock" | "random" | "crypto">,
    ) {
        // draw the owner's credential
        this.token = environment.random.bytes(OWNER_TOKEN_BYTES).toHex();

        // set the machine, its device, its peers and the clock
        this.subject = principal.machine.reference(Scope.universe.id, machineId);
        this.machineId = machineId;
        this.#credential = device.credential;
        this.#device = device;
        this.#machine = machine;
        this.#clock = environment.clock;
        this.#crypto = environment.crypto;
    }

    /** Authenticate a request to the device's partition, the owner acting as the device. */
    async authenticateDevice(
        request: Request,
        audience: PackageId,
    ): Promise<Authentication | undefined> {
        // take the owner as the device, anyone else as they authenticate
        const verified = await this.authenticate(request, audience);
        const device = principal.device.reference(Scope.universe.id, this.#device.id);

        return verified?.credential.kind === "local"
            ? { ...verified, caller: Caller.of(device) }
            : verified;
    }

    /** Authenticate a request to a package's service in the partition it targets, if any. */
    async authenticate(
        request: Request,
        audience: PackageId,
        partition?: PartitionId,
    ): Promise<Authentication | undefined> {
        // take a call without a bearer credential as anonymous, access deciding what it reaches
        const token = Bearer.read(request.headers);
        if (token === undefined) {
            return undefined;
        }

        // verify a caller on another machine by the universe's token, a local client by its proof
        if (token.split(".").length === JWT_PARTS && !Proof.is(token)) {
            return this.#authenticateRemote(request, audience, partition);
        }

        // take the owner from this machine outside any browser, comparing the credentials' digests
        const [given, expected] = await Promise.all([
            Digest.of(token, this.#crypto),
            Digest.of(this.token, this.#crypto),
        ]);
        const isOwner = given === expected;
        const isLocal = Authenticator.#isLocal(request);
        const now = this.#clock.now();
        if (isOwner && isLocal) {
            return {
                caller: Caller.of(this.subject),
                credential: {
                    kind: "local",
                    id: this.machineId,
                    audience,
                    verifiedAt: now,
                    expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
                },
            };
        }

        // take a person's own client from this computer by its key's proof alone
        if (isOwner || !isLocal || !Proof.is(token)) {
            throw MachineError.of(
                "FOREIGN_ORIGIN",
                "the credential is not accepted from this origin",
            );
        }
        const clientId = schema.identifier("client").parse(await this.#credential.verify(token));
        const person = await this.#person(clientId);

        // act as the person inside the native device, bound to the partition the call targets
        const native = principal.device.reference(Scope.universe.id, this.#device.id);

        return {
            caller: { ...Caller.of(person), groups: [{ ...native, relation: CONTAINED }] },
            credential: {
                kind: "session",
                id: clientId,
                client: clientId,
                audience,
                ...(partition === undefined ? {} : { scope: partition }),
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
            },
        };
    }

    /** Read the person signed in here through a client, refusing one signed out. */
    async #person(clientId: string): Promise<Subject> {
        // read the person signed in with the client
        const registration = schema.identifier("client").parse(clientId);
        const [signedIn] = await this.#device.database
            .select({ subject: login.table.subjectId })
            .from(login.table)
            .where(
                and(
                    login.inScope(this.#device.id),
                    eq(login.table.clientId, registration),
                    eq(login.table.status, "authenticated"),
                ),
            );
        if (signedIn === undefined) {
            throw MachineError.of("SIGN_IN_REQUIRED", "no person signed in through the client");
        }

        return principal.user.reference(Scope.universe.id, signedIn.subject);
    }

    /** Authenticate a caller on another machine while its key stands. */
    async #authenticateRemote(
        request: Request,
        audience: PackageId,
        partition: PartitionId | undefined,
    ): Promise<Authentication> {
        // require a token bound to the partition a call goes into, a peer machine's own calls unbound
        const verified = await this.#machine.verify(request, audience, partition);
        const { credential } = verified;
        const isPeer = credential.kind === "machine-key";
        if (credential.scope === undefined && (partition !== undefined || !isPeer)) {
            throw MachineError.of("TOKEN_UNBOUND", UNBOUND_REFUSAL);
        }
        // require a space's token bound to it, or to the account the space is the home of
        else if (
            credential.scope !== undefined &&
            credential.kind !== "installation" &&
            partition !== undefined &&
            space.identifies(partition)
        ) {
            await (
                await this.#machine.space(space.identifier(partition))
            ).host.requirePartition(verified);
        }
        // require a machine's or a device's token bound to it
        else if (
            credential.scope !== undefined &&
            credential.kind !== "installation" &&
            credential.scope !== partition
        ) {
            throw MachineError.of(
                "FOREIGN_PARTITION_TOKEN",
                "the universe token is bound to another partition",
            );
        }

        // require a peer machine's key to stand
        if (isPeer) {
            await this.#machine.requireStanding(verified);
        }

        return verified;
    }

    /** Authorize a call to the machine's own packages, their own procedures to the owner alone. */
    async authorizeMachine(call: AuthorizedCall): Promise<void> {
        // refuse what spaces' procedures refuse and the machine's own procedures to clients
        await this.authorizeSpace(call);
        const credential = call.request.credential();
        const { permission } = call.access;
        const isOwn = typeof permission === "object" && "principals" in permission;
        if (credential.kind !== "local" && isOwn) {
            throw MachineError.of(
                "OWNER_REQUIRED",
                "only the owner can call the machine's own procedures",
            );
        }
    }

    /** Authorize a call to a space's packages, refusing revoked credentials and non-owners. */
    async authorizeSpace({ request, access }: AuthorizedCall): Promise<void> {
        // pass anyone calling a public procedure
        if (access.authentication === "public") {
            return;
        }

        // recheck a person signed in here, whose session binds no space as other tokens do
        const credential = request.credential();
        if (credential.kind === "session" && credential.scope === undefined) {
            await this.#person(credential.id);
        }

        // refuse machine procedures to everyone but the owner
        if (credential.kind !== "local" && access.authentication === "machine") {
            throw MachineError.of("OWNER_REQUIRED", "only the owner can call machine procedures");
        }
    }

    /** Report whether a request comes from this computer outside any browser, by its first hop. */
    static #isLocal(request: Request): boolean {
        const peer = Forwarded.first(request).for;

        return (
            (peer === undefined || HostAddress.isLoopback(peer)) &&
            request.headers.get("origin") === null
        );
    }
}

/** A procedure call as authorization reads it: its caller and its access. */
interface AuthorizedCall {
    /** The call's request, authenticating its caller. */
    readonly request: {
        readonly credential: () => Credential;
    };
    /** The authentication and permission the procedure requires. */
    readonly access: Pick<
        ProcedureContext<RequestContext>["access"],
        "authentication" | "permission"
    >;
}

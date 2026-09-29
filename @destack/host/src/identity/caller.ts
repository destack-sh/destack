import { principal, type Subject } from "@destack/access";
import { Scope } from "@destack/sync";
import { DeviceProof } from "@destack/account/object";
import type { DatabaseConnection } from "@destack/db";
import { Condition } from "@destack/db/query";
import { identifier } from "@destack/schema";
import type { PackageId } from "@destack/package";
import { Caller, CALLER_LIFETIME_MILLISECONDS } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { originalRequest } from "@destack/service/request";
import type { ProcedureCall, ServiceContext } from "@destack/service/server";
import type { Directory } from "@destack/directory";
import type { connect } from "../client/index.ts";
import { HostKey } from "../object/host.ts";
import { HOST_PROOF_SCHEME } from "./identity.ts";

/** The authorization header of a signed request. */
const PROOF_AUTHORIZATION = new RegExp(`^${HOST_PROOF_SCHEME} (\\S+)$`);

/** The credential a verified host presented. */
export interface HostCredential {
    /** The credential's kind. */
    readonly kind: "host-key";
    /** The key that signed the proof. */
    readonly id: string;
    /** The host the key belongs to. */
    readonly hostId: string;
}

/** A host verified by a proof from one of its active keys, acting for itself and its region. */
export class HostCaller extends Caller<HostCredential> {
    /** Report whether a request carries a host proof. */
    static accepts(request: Request): boolean {
        return request.headers.get("authorization")?.startsWith(`${HOST_PROOF_SCHEME} `) === true;
    }

    /** Refuse a procedure requiring host credentials to a caller presenting no host key. */
    static async authorize({ context, access }: ProcedureCall<ServiceContext>): Promise<void> {
        if (access.authentication === "host" && !(context.caller instanceof HostCaller)) {
            throw new ServiceError("FORBIDDEN", { message: "the procedure requires a host key" });
        }
    }

    /** Verify a request's host proof once against the host's active keys, for a package. */
    static async authenticate(
        request: Request,
        database: DatabaseConnection,
        audience: PackageId,
        now = Date.now(),
    ): Promise<HostCaller> {
        // read the proof and refuse cookies beside it
        const proof = await HostCaller.#read(request);

        // find the standing key the proof refers to
        const [found] = await HostKey.select(
            database,
            Condition.all(Condition.eq("thumbprint", proof.thumbprint), HostKey.standing(now)),
        );
        if (found === undefined) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "host proof refers to no active key",
            });
        }
        await HostCaller.#refuseInvalid(async () => {
            await proof.verify(found.publicKey, found.hostId, now, originalRequest(request));
            await proof.consume(database, now);
        });

        // refuse a host its account disabled, whose keys stay suspended
        if (found.suspendedAt !== null) {
            throw new ServiceError("UNAUTHORIZED", { message: "host is disabled" });
        }

        // act as the host principal, and for the region it serves
        const subject = principal.host.reference(found.accountId, found.hostId);
        const regions: Subject[] =
            found.regionId === null
                ? []
                : [principal.region.reference(Scope.universe.id, found.regionId)];

        return new HostCaller({
            credential: { kind: "host-key", id: found.id, hostId: found.hostId },
            audience,
            verifiedAt: now,
            expiresAt: now + CALLER_LIFETIME_MILLISECONDS,
            subject,
            subjects: [subject, ...regions],
        });
    }

    /** Verify a peer host's proof once against its keys the host service serves, for a package. */
    static async peer(
        request: Request,
        hosts: ReturnType<typeof connect>,
        directory: Directory,
        database: DatabaseConnection,
        audience: PackageId,
        now = Date.now(),
    ): Promise<HostCaller> {
        // read the proof and refuse cookies beside it
        const proof = await HostCaller.#read(request);

        // find the account of the proof's host through its cell in the directory
        const cell = await directory.cell(proof.deviceId);
        if (cell === undefined) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "host proof refers to no active key",
            });
        }

        // find the proof's active key among the keys of the hosts this host may verify, none when it may verify none there
        const keys = await hosts.hostKey
            .list({
                accountId: identifier("account").parse(cell.scope),
                where: Condition.all(
                    Condition.eq("parentId", proof.deviceId),
                    Condition.eq("thumbprint", proof.thumbprint),
                    HostKey.authenticates(now),
                ),
            });
        const [key] = keys.items;
        if (key === undefined) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "host proof refers to no active key",
            });
        }
        await HostCaller.#refuseInvalid(async () => {
            await proof.verify(key.publicKey, proof.deviceId, now, originalRequest(request));
            await proof.consume(database, now);
        });

        // act as the host principal
        const subject = principal.host.reference(cell.scope, proof.deviceId);

        return new HostCaller({
            credential: { kind: "host-key", id: key.id, hostId: proof.deviceId },
            audience,
            verifiedAt: now,
            expiresAt: now + CALLER_LIFETIME_MILLISECONDS,
            subject,
            subjects: [subject],
        });
    }

    /** Read a request's host proof and refuse a request with cookies beside it. */
    static #read(request: Request): Promise<DeviceProof> {
        // take the proof the authorization header carries
        const authorization = request.headers.get("authorization") ?? "";
        const match = PROOF_AUTHORIZATION.exec(authorization);
        if (match === null || request.headers.has("cookie")) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid host proof" });
        }

        return HostCaller.#refuseInvalid(async () => DeviceProof.read(match[1]!));
    }

    /** Run a step of checking a proof, reporting an invalid or replayed proof as unauthorized. */
    static async #refuseInvalid<Value>(step: () => Promise<Value>): Promise<Value> {
        try {
            return await step();
        } catch (error) {
            // keep the proof's reason, as an authentication failure
            if (error instanceof ServiceError && error.code === "BAD_REQUEST") {
                throw new ServiceError("UNAUTHORIZED", { message: error.message, cause: error });
            }
            throw error;
        }
    }
}

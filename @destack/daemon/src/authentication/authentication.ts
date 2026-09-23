import { timingSafeEqual } from "node:crypto";
import { Caller, CALLER_LIFETIME_MS } from "@destack/service/authentication";
import type { Subject } from "@destack/access";
import type { PackageId } from "@destack/package";
import type { ProcedureCall, ServiceContext } from "@destack/service/server";
import { ServiceError } from "@destack/service";
import { CredentialRegistry } from "./credential.ts";

/** Local owner authentication and restricted application credentials. */
export class Authentication {
    /** Private owner credential published to local clients. */
    readonly token = crypto.getRandomValues(new Uint8Array(32)).toHex();
    /** Encoded owner header used for constant-time comparison. */
    readonly #expected = new TextEncoder().encode(`Bearer ${this.token}`);
    /** Process-local application authorizations. */
    readonly credentials = new CredentialRegistry();
    /** Local host authority, independent of any signed-in person. */
    readonly subject: Subject;
    /** Host administering local operations. */
    readonly hostId: string;
    /** Receiving daemon package. */
    readonly audience: PackageId;

    /** Bind private local administration to the persistent host. */
    constructor(hostId: string, audience: PackageId) {
        this.subject = { kind: "host", authority: hostId, id: hostId };
        this.hostId = hostId;
        this.audience = audience;
    }

    /** Authenticate local credentials without accepting ambient browser requests. */
    authenticate(request: Request): Caller {
        // restrict local credentials to same-machine non-browser requests
        const url = new URL(request.url);
        const authorization = new TextEncoder().encode(request.headers.get("authorization") ?? "");
        if (url.hostname !== "127.0.0.1" || request.headers.has("origin")) {
            throw new ServiceError("UNAUTHORIZED");
        }

        // distinguish the private owner credential from issued application credentials
        const isOwner =
            authorization.length === this.#expected.length &&
            timingSafeEqual(authorization, this.#expected);
        const bearer = request.headers.get("authorization");
        if (!bearer?.startsWith("Bearer ")) {
            throw new ServiceError("UNAUTHORIZED");
        }
        const credentialId = this.credentials.id(bearer.slice(7));
        const client = isOwner ? undefined : this.credentials.get(credentialId);
        const now = Date.now();

        return new Caller({
            credential: isOwner
                ? { kind: "local", id: this.hostId }
                : {
                      kind: "client",
                      id: credentialId,
                      packageId: client!.packageId,
                  },
            permissions: client?.permissions.map((permission) => ({
                ...permission,
                scope: this.hostId,
            })),
            audience: this.audience,
            scope: this.hostId,
            subject: this.subject,
            subjects: [this.subject],
            verifiedAt: now,
            expiresAt: now + CALLER_LIFETIME_MS,
        });
    }

    /** Recheck revocation and exclude owner-only procedures from application grants. */
    authorize({ context, access }: ProcedureCall<ServiceContext>): void {
        const caller = context.requireCaller();
        const credential = caller.credential as { kind: string; id: string };
        if (credential.kind === "client") {
            this.credentials.get(credential.id);
            if (access.authentication === "host" || access.permission === null) {
                throw new ServiceError("FORBIDDEN");
            }
        }
    }
}

import { createHash } from "node:crypto";
import { ServiceError } from "@destack/service";
import type { ClientAuthorization } from "../service/credential.ts";
import type { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { createCredential, revokeCredential } from "../audit/index.ts";

/** Restricted local credentials whose authorization ends with the daemon process. */
export class CredentialRegistry {
    /** Authorizations indexed by a digest of the bearer credential. */
    readonly #credentials = new Map<string, ClientAuthorization>();

    /** Issue a credential after the host owner approves its complete authorization. */
    async create(
        authorization: ClientAuthorization,
        audit: AuditRecorder<DatabaseConnection>,
    ): Promise<{ token: string }> {
        // record the exact grant before making it available to callers
        const token = crypto.getRandomValues(new Uint8Array(32)).toHex();
        const id = this.id(token);
        const attempt = audit.begin(createCredential, {
            targets: { credential: { type: "credential", id } },
            details: authorization,
        });
        await audit.append(attempt);
        this.#credentials.set(id, structuredClone(authorization));

        // withdraw an issued credential when its completion cannot be persisted
        try {
            await audit.append(audit.complete(attempt, { outcome: "success" }));
        } catch (error) {
            this.#credentials.delete(id);
            throw error;
        }

        return { token };
    }

    /** Derive the public reference used by request contexts. */
    id(token: string): string {
        return createHash("sha256").update(token).digest("hex");
    }

    /** Require an active credential on every authorization check. */
    get(id: string): ClientAuthorization {
        const authorization = this.#credentials.get(id);
        if (!authorization) {
            throw new ServiceError("UNAUTHORIZED");
        }

        return authorization;
    }

    /** Revoke a credential without disclosing whether it existed. */
    async revoke(token: string, audit: AuditRecorder<DatabaseConnection>): Promise<void> {
        // record intent before revocation without persisting the bearer secret
        const id = this.id(token);
        const attempt = audit.begin(revokeCredential, {
            targets: { credential: { type: "credential", id } },
            details: {},
        });
        await audit.append(attempt);

        // keep revocation effective even if completion persistence fails
        this.#credentials.delete(id);
        await audit.append(audit.complete(attempt, { outcome: "success" }));
    }
}

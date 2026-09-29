import { DeviceProof } from "@destack/account/object";
import type { DatabaseConnection } from "@destack/db";
import { Condition } from "@destack/db/query";
import { ServiceError } from "@destack/service/error";
import { HostKey } from "../object/host.ts";

/** A host's key-signed assertion that grants it access tokens: a proof bound to the grant's request, spent once. */
export class HostAssertion {
    /** Verify an assertion bound to a request against the host's standing keys, spending it, and return its key with the host. */
    static async verify(
        assertion: string,
        request: { readonly method: string; readonly url: string },
        database: DatabaseConnection,
        now = Date.now(),
    ) {
        // read the proof and find the standing key it refers to
        const proof = await HostAssertion.#refuseInvalid(async () => DeviceProof.read(assertion));
        const [found] = await HostKey.select(
            database,
            Condition.all(Condition.eq("thumbprint", proof.thumbprint), HostKey.standing(now)),
        );
        if (found === undefined) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "host assertion refers to no active key",
            });
        }

        // verify it for the request and spend it
        await HostAssertion.#refuseInvalid(async () => {
            await proof.verify(found.publicKey, found.hostId, now, request);
            await proof.consume(database, now);
        });

        // refuse a host its account disabled, whose keys stay suspended
        if (found.suspendedAt !== null) {
            throw new ServiceError("UNAUTHORIZED", { message: "host is disabled" });
        }

        return found;
    }

    /** Run a step of checking an assertion, reporting an invalid or replayed one as unauthorized. */
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

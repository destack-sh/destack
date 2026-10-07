import { principal } from "@destack/access";
import type { Identifier } from "@destack/schema";
import { Authentication, type TokenVerifier } from "@destack/service/authentication";
import { RELAY_PACKAGE } from "../server/index.ts";

/** The account the test machines belong to. */
const ACCOUNT = "account-01996ab0-0000-7000-8000-0000000000a1";

/** Verify test tokens of a machine, each naming its lifetime in milliseconds, as the universe's verifier admits them. */
export function machineTokens(machineId: Identifier<"machine">): Pick<TokenVerifier, "verify"> {
    return {
        verify: async (token) => {
            const subject = principal.machine.reference(ACCOUNT, machineId);
            const now = Date.now();

            return new Authentication({
                subject,
                subjects: [subject],
                credential: { kind: "machine-key", id: `key-${machineId}` },
                audience: RELAY_PACKAGE.id,
                verifiedAt: now,
                expiresAt: now + Number(token),
            });
        },
    };
}

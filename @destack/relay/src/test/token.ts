import { Caller, principal } from "@destack/access";
import type { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import { type TokenVerifier } from "@destack/runtime/authentication";
import relayPackage from "../package.ts";

/** Verify a machine's test tokens, each with its lifetime in milliseconds. */
export function machineTokens(machineId: Identifier<"machine">): Pick<TokenVerifier, "verify"> {
    return {
        verify: async (token, now) => {
            const subject = principal.machine.reference(Scope.universe.id, machineId);

            return {
                caller: Caller.of(subject),
                credential: {
                    kind: "machine-key",
                    id: `key-${machineId}`,
                    audience: relayPackage.id,
                    verifiedAt: now,
                    expiresAt: now + Number(token),
                },
            };
        },
    };
}

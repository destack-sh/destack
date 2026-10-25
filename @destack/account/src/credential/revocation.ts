import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** Withdrawals of credentials and machines, as the time each was revoked at. */
export const Revocation = {
    /** Refuse a withdrawn record, reading its noun in the message, and return a standing one. */
    requireUnrevoked<Target extends { readonly revokedAt: number | null }>(
        target: Target,
        noun: string,
    ): Target {
        if (target.revokedAt !== null) {
            throw AccountError.of("CREDENTIAL_REVOKED", `${noun} is revoked`);
        }

        return target;
    },
};

import type { Call } from "@destack/object";
import { ServiceError } from "@destack/service/error";

/** Refuse a withdrawn object, reading its noun in the message. */
export function requireUnrevoked(target: { readonly revokedAt?: unknown }, noun: string): void {
    if (target.revokedAt !== null) {
        throw new ServiceError("CONFLICT", { message: `${noun} is revoked` });
    }
}

/** Withdraw a call's target now, refusing to withdraw it twice. */
export function revokeOnce(call: Call, noun: string): Promise<unknown> {
    requireUnrevoked(call.target!, noun);

    return call.revise({ revokedAt: call.now });
}

import { PackageId } from "@destack/package";
import type { Subject } from "@destack/sync";
import { ResourceContext } from "@destack/resource/context";
import { Authentication, type AuthenticationClaims } from "../authentication/index.ts";
import type { Reconciliation } from "../control/index.ts";
import { type CallKey } from "../request/index.ts";
import { ServiceContext } from "../server/index.ts";

/** The package a hand-built test context addresses. */
const AUDIENCE = PackageId.parse("package-01996ab0-0000-7000-8000-00000000c0de");

/** How long a hand-built test caller stays current: a minute, above any test's timeout. */
const LIFETIME_MILLISECONDS = 60_000;

/** The deployment key of tests, one per test process. */
const TEST_KEY = crypto.subtle.generateKey({ name: "HMAC", hash: "SHA-256" }, false, ["sign"]);

/** Read the deployment key tests fingerprint sensitive inputs under. */
export const testCallKey: CallKey = () => TEST_KEY;

/** How a hand-built test caller signs in, and the signal and clock of its requests. */
export interface SubjectContextOptions extends Pick<
    AuthenticationClaims,
    "delegates" | "assurance" | "permissions" | "attributes"
> {
    /** The signal ending the request. */
    readonly signal?: AbortSignal;
    /** Read the current time calls run at, the system clock by default. */
    readonly clock?: () => number;
}

/** Build the request context of a subject calling in a scope or in none, ending with a signal and timed by a clock. */
export function subjectContext(
    subject: Subject,
    scope: string | undefined,
    options: SubjectContextOptions = {},
): ServiceContext {
    // sign the subject in for a minute with its delegates, assurance and restrictions
    const { signal, clock, ...caller } = options;
    const now = (clock ?? Date.now)();
    const authentication = new Authentication({
        ...caller,
        credential: { kind: "session", id: subject.id },
        audience: AUDIENCE,
        subject,
        subjects: [subject],
        verifiedAt: now,
        expiresAt: now + LIFETIME_MILLISECONDS,
    });
    const request = new Request("https://test.local", signal === undefined ? {} : { signal });

    return new ServiceContext(request, {
        audience: AUDIENCE,
        ...(scope === undefined ? {} : { scope }),
        authentication,
        resources: new ResourceContext(),
        ...(clock === undefined ? {} : { clock }),
    });
}

/** Build a reconciliation outside a control loop: stopped by a signal, its key never changing. */
export function reconciliation(signal: AbortSignal = new AbortController().signal): Reconciliation {
    return { signal, changed: () => new Promise<void>(() => {}) };
}

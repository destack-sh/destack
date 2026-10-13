import { TOKEN_LIFETIME_MILLISECONDS } from "../authentication/index.ts";
import type { Caller } from "@destack/access";
import { PackageId } from "@destack/package";
import type { Subject } from "@destack/sync";
import { ResourceContext } from "@destack/resource/context";
import type { Reconciliation } from "../control/index.ts";
import { type CallKey } from "../request/index.ts";
import { ServiceContext } from "../server/index.ts";
import { Clock } from "@destack/schema";

/** The package a hand-built test context addresses. */
const AUDIENCE = PackageId.parse("package-01996ab0-0000-7000-8000-00000000c0de");

/** The deployment key of tests, one per test process. */
const TEST_KEY = crypto.subtle.generateKey({ name: "HMAC", hash: "SHA-256" }, false, ["sign"]);

/** Read the deployment key tests fingerprint sensitive inputs under. */
export const testCallKey: CallKey = () => TEST_KEY;

/** How a hand-built test caller signs in, and the signal and clock of its requests. */
export interface SubjectContextOptions extends Pick<
    Caller,
    "delegates" | "grant" | "assurance" | "attributes"
> {
    /** The signal ending the request. */
    readonly signal?: AbortSignal;
    /** The clock calls run at, the system clock by default. */
    readonly clock?: Clock;
}

/** Build the request context of a subject calling in a scope or in none. */
export function subjectContext(
    subject: Subject,
    scope: string | undefined,
    options: SubjectContextOptions = {},
): ServiceContext {
    // sign the subject in for a minute with its delegates, grant and assurance
    const { signal, clock = Clock.system, ...restrictions } = options;
    const now = clock.now();
    const caller: Caller = {
        ...restrictions,
        credential: {
            kind: "session",
            id: subject.id,
            audience: AUDIENCE,
            verifiedAt: now,
            expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
        },
        subject,
        subjects: [subject],
    };
    const request = new Request("https://test.local", signal === undefined ? {} : { signal });

    return new ServiceContext(request, {
        audience: AUDIENCE,
        ...(scope === undefined ? {} : { scope }),
        caller,
        resources: new ResourceContext(),
        clock,
    });
}

/** Build a reconciliation outside a control loop: stopped by a signal, its key never changing. */
export function reconciliation(signal: AbortSignal = new AbortController().signal): Reconciliation {
    return { signal, changed: () => new Promise<void>(() => {}) };
}

import type { Subject } from "@destack/access";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Caller } from "../authentication/index.ts";
import type { Reconciliation } from "../control/index.ts";
import type { JournalKey } from "../database/index.ts";
import { ServiceContext } from "../server/index.ts";

/** The package a hand-built test context addresses. */
const AUDIENCE = PackageId.parse("package-01996ab0-0000-7000-8000-00000000c0de");

/** How long a hand-built test caller stays current: a minute, above any test's timeout. */
const LIFETIME_MILLISECONDS = 60_000;

/** The deployment key of tests, one per test process. */
const TEST_KEY = crypto.subtle.generateKey({ name: "HMAC", hash: "SHA-256" }, false, ["sign"]);

/** Read the deployment key tests fingerprint sensitive inputs under. */
export const testJournalKey: JournalKey = () => TEST_KEY;

/** Build the request context of a subject calling in a scope, ending with a signal. */
export function subjectContext(
    subject: Subject,
    scope: string,
    signal?: AbortSignal,
): ServiceContext {
    // sign the subject in for a minute
    const now = Date.now();
    const caller = new Caller({
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
        scope,
        caller,
        resources: new ResourceContext(),
    });
}

/** Build a reconciliation outside a control loop: stopped by a signal, its key never changing. */
export function reconciliation(signal: AbortSignal = new AbortController().signal): Reconciliation {
    return { signal, changed: () => new Promise<void>(() => {}) };
}

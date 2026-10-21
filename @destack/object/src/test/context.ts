import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import type { Caller, Credential } from "@destack/access";
import { PackageId } from "@destack/package";
import type { Subject } from "@destack/sync";
import { ResourceContext } from "@destack/resource/context";
import { type CallKey } from "../audit/key.ts";
import { RequestContext } from "../server/index.ts";
import { Clock } from "@destack/environment";

/** The package a hand-built test context addresses. */
const AUDIENCE = PackageId.parse("package-01996ab0-0000-7000-8000-00000000c0de");

/** The deployment key of tests, one per test process. */
const TEST_KEY = crypto.subtle.generateKey({ name: "HMAC", hash: "SHA-256" }, false, ["sign"]);

/** Read the deployment key tests fingerprint sensitive inputs under. */
export const testCallKey: CallKey = () => TEST_KEY;

/** How a hand-built test caller signs in, and the signal and clock of its requests. */
export interface SubjectContextOptions
    extends
        Partial<Pick<Caller, "actors" | "groups" | "contacts" | "grants">>,
        Partial<Pick<Credential, "assurance" | "attributes">> {
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
): RequestContext {
    // sign the subject in for a minute with its actors, groups, grants and assurance
    const { signal, clock = Clock.system, assurance, attributes, ...restrictions } = options;
    const now = clock.now();
    const caller: Caller = {
        actors: [],
        groups: [],
        contacts: [],
        grants: [],
        ...restrictions,
        subject,
    };
    const credential: Credential = {
        kind: "session",
        id: subject.id,
        audience: AUDIENCE,
        verifiedAt: now,
        expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
        ...(assurance === undefined ? {} : { assurance }),
        ...(attributes === undefined ? {} : { attributes }),
    };
    const request = new Request("https://test.local", signal === undefined ? {} : { signal });

    return new RequestContext(request, {
        audience: AUDIENCE,
        ...(scope === undefined ? {} : { scope }),
        authentication: { caller, credential },
        resources: new ResourceContext(),
        clock,
    });
}

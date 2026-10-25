import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import type { Caller, Credential } from "@destack/access";
import { PackageId } from "@destack/package";
import type { Subject } from "@destack/sync";
import { ResourceContext } from "@destack/resource/context";
import { CallKey } from "../audit/key.ts";
import { RequestContext } from "../server/index.ts";
import type { Environment } from "@destack/environment";
import { BOOKMARK_HEADER, type Bookmark } from "../procedure/bookmark.ts";

/** The package a hand-built test context addresses. */
const AUDIENCE = PackageId.parse("package-01996ab0-0000-7000-8000-00000000c0de");

/** The deployment key's secret bytes of tests: fixed, as one deployment's tests share it. */
const TEST_KEY_BYTES = new Uint8Array(32).fill(1);

/** Read the deployment key tests fingerprint sensitive inputs under, on the store's WebCrypto. */
export const testCallKey: CallKey = (crypto) => CallKey.import(TEST_KEY_BYTES, crypto);

/** How a hand-built test caller signs in, and the signal and world of its requests. */
export interface SubjectContextOptions
    extends
        Partial<Pick<Caller, "actors" | "groups" | "contacts" | "grants">>,
        Partial<Pick<Credential, "assurance" | "attributes" | "client">> {
    /** The signal ending the request. */
    readonly signal?: AbortSignal;
    /** The world calls run in. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "random">;
    /** The writes the request has seen, none by default. */
    readonly bookmark?: Bookmark;
}

/** Build the request context of a subject calling in a scope or in none. */
export function subjectContext(
    subject: Subject,
    scope: string | undefined,
    options: SubjectContextOptions,
): RequestContext {
    // sign the subject in for a minute with its actors, groups, grants, assurance and client
    const { signal, environment, assurance, attributes, client, bookmark, ...restrictions } =
        options;
    const { clock } = environment;
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
        ...(client === undefined ? {} : { client }),
    };
    const request = new Request("https://test.local", {
        ...(signal === undefined ? {} : { signal }),
        ...(bookmark === undefined ? {} : { headers: { [BOOKMARK_HEADER]: bookmark.format() } }),
    });

    return new RequestContext(request, {
        audience: AUDIENCE,
        ...(scope === undefined ? {} : { scope }),
        authentication: { caller, credential },
        resources: new ResourceContext(),
        environment,
        lapse: RequestContext.lapse(credential, clock, request.signal),
    });
}

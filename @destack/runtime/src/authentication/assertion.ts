import { PublicKey } from "@destack/identity";
import type { PackageId } from "@destack/package";
import { Digest, schema } from "@destack/schema";
import type { Subject } from "@destack/sync";
import {
    type Caller,
    CLOCK_TOLERANCE_MILLISECONDS,
    KEY_ASSURANCE,
    principal,
} from "@destack/access";
import { decodeJwt, decodeProtectedHeader, errors, jwtVerify, SignJWT } from "jose";
import { CallError, isCallError } from "../error/index.ts";
import { Bearer } from "./bearer.ts";
import { INSTALLATION_KEY, MACHINE_KEY, SPACE_KEY } from "./token.ts";

/** The JWT type of a key-signed assertion, after RFC 7523bis's explicit typing. */
export const ASSERTION_TYPE = "client-authentication+jwt";

/** The longest an assertion lives from its issue time: five minutes, short as RFC 7523 suggests. */
export const ASSERTION_LIFETIME_MILLISECONDS = 300_000;

/** How long a signed assertion lives by default: a minute, one request and its retries. */
const SIGNED_LIFETIME_MILLISECONDS = 60_000;

/** A principal's published key that signs its assertions. */
export interface AssertionKey {
    /** The key's identifier, which an assertion's header refers to as its kid. */
    readonly id: string;
    /** The principal the key signs as: a machine, an installation or a space. */
    readonly subject: Subject;
    /** The public key. */
    readonly publicKey: PublicKey;
}

/** A caller an assertion authenticated: its subject and credential, both present. */
type AssertedCaller = Caller & Required<Pick<Caller, "subject" | "credential">>;

/** Key-signed assertions principals exchange at a token endpoint (RFC 7523 jwt-bearer). */
export const Assertion = {
    /** Report whether a bearer token is a key-signed assertion, by its JWT type. */
    is(token: string): boolean {
        try {
            return decodeProtectedHeader(token).typ === ASSERTION_TYPE;
        } catch {
            return false;
        }
    },

    /** Sign an assertion of a principal under a key for a token endpoint, living a minute. */
    async sign(
        privateKey: CryptoKey,
        key: AssertionKey,
        endpoint: string,
        now: number,
        subject: string = key.subject.id,
    ): Promise<string> {
        const [algorithm] = PublicKey.algorithms(key.publicKey);
        const issuedAt = Math.floor(now / 1000);

        return new SignJWT({ jti: crypto.randomUUID() })
            .setProtectedHeader({ alg: algorithm, typ: ASSERTION_TYPE, kid: key.id })
            .setIssuer(key.subject.id)
            .setSubject(subject)
            .setAudience(endpoint)
            .setIssuedAt(issuedAt)
            .setExpirationTime(issuedAt + SIGNED_LIFETIME_MILLISECONDS / 1000)
            .sign(privateKey);
    },
};

/** Verify a token endpoint's key-signed assertions as their principals, spending each once. */
export class AssertionVerifier {
    /** The verifier configuration. */
    readonly options: AssertionVerifierOptions;

    /** Create the verifier. */
    constructor(options: AssertionVerifierOptions) {
        this.options = options;
    }

    /** Verify and spend an assertion at a time, answering its principal as the caller. */
    async verify(assertion: string, now: number): Promise<AssertedCaller> {
        // find the standing key of the claimed issuer the header refers to
        const { kid } = refused(() => decodeProtectedHeader(assertion));
        const { iss } = refused(() => decodeJwt(assertion));
        const key =
            kid === undefined || iss === undefined
                ? undefined
                : await this.options.keys(kid, iss, now);
        if (key === undefined) {
            throw new CallError("UNAUTHORIZED", { message: "assertion has no standing key" });
        }

        // verify the signature, the type, and the claims of the key's principal and this endpoint
        const verifying = jwtVerify(assertion, await PublicKey.import(key.publicKey), {
            algorithms: PublicKey.algorithms(key.publicKey),
            typ: ASSERTION_TYPE,
            issuer: key.subject.id,
            audience: this.options.endpoint,
            requiredClaims: ["iat", "exp", "jti"],
            maxTokenAge: ASSERTION_LIFETIME_MILLISECONDS / 1000,
            clockTolerance: CLOCK_TOLERANCE_MILLISECONDS / 1000,
            currentDate: new Date(now),
        });
        const { payload } = await verifying.catch((error: unknown) => {
            throw error instanceof errors.JWTExpired
                ? new CallError("UNAUTHORIZED", { message: "assertion expired", cause: error })
                : Bearer.refusal(error, "invalid assertion");
        });
        const { sub, iat, exp, jti } = payload;
        if (sub === undefined || iat === undefined || exp === undefined || jti === undefined) {
            throw new CallError("UNAUTHORIZED", { message: "invalid assertion" });
        } else if ((exp - iat) * 1000 > ASSERTION_LIFETIME_MILLISECONDS) {
            throw new CallError("UNAUTHORIZED", {
                message: "assertion lifetime exceeds five minutes",
            });
        }

        // spend it once under its key, refusing one spent before, and answer the key's principal
        const expiresAt = exp * 1000;
        const digest = await Digest.of(JSON.stringify([key.id, jti]));
        await this.options.spend(digest, expiresAt, now).catch((error: unknown) => {
            throw isCallError(error) && error.code === "CONFLICT"
                ? new CallError("UNAUTHORIZED", {
                      message: "assertion was used before",
                      cause: error,
                  })
                : error;
        });

        return asserted(key, sub, this.options.audience, now, expiresAt);
    }
}

/** The configuration of an assertion verifier. */
export interface AssertionVerifierOptions {
    /** The token endpoint's URL, every assertion's audience (RFC 7523 3). */
    readonly endpoint: string;
    /** The package serving the token endpoint, the audience of the callers it verifies. */
    readonly audience: PackageId;
    /** Read an issuer's standing key an assertion's header refers to, absent for none. */
    readonly keys: (id: string, issuer: string, now: number) => Promise<AssertionKey | undefined>;
    /** Spend an assertion's digest once until it expires, refusing a repeat as a conflict. */
    readonly spend: (digest: Digest, expiresAt: number, now: number) => Promise<void>;
}

/** Describe an assertion's principal as the caller it authenticated until it expires. */
function asserted(
    key: AssertionKey,
    sub: string,
    audience: PackageId,
    now: number,
    expiresAt: number,
): AssertedCaller {
    // take the key's own machine, space or installation, or an installation its space signs for
    const subject = assertedSubject(key, sub);
    const isMachine = principal.machine.is(subject);
    const isSpace = principal.space.is(subject);

    return {
        subject,
        subjects: [subject],
        assurance: { level: KEY_ASSURANCE, authenticatedAt: now },
        credential: {
            kind: isMachine ? MACHINE_KEY : isSpace ? SPACE_KEY : INSTALLATION_KEY,
            id: key.id,
            audience,
            // bind an installation to its space, while a machine or a space acts as itself
            ...(isMachine || isSpace ? {} : { scope: subject.scope }),
            verifiedAt: now,
            expiresAt,
        },
    };
}

/** Read the principal an assertion's subject claim asserts under a key, refusing another. */
function assertedSubject(key: AssertionKey, sub: string): Subject {
    // take the key's own machine, space or installation
    const { subject } = key;
    const installation = schema.identifier("installation").safeParse(sub);
    if (
        sub === subject.id &&
        (principal.machine.is(subject) ||
            principal.space.is(subject) ||
            principal.installation.is(subject))
    ) {
        return subject;
    }
    // take an installation of the space signing for it
    else if (principal.space.is(subject) && installation.success) {
        return principal.installation.reference(subject.id, installation.data);
    }

    // refuse any other subject
    throw new CallError("UNAUTHORIZED", {
        message: "assertion subject is outside its key's authority",
    });
}

/** Read an assertion's header, refusing one that is not a compact JWS. */
function refused<Value>(read: () => Value): Value {
    try {
        return read();
    } catch (error) {
        if (!(error instanceof errors.JOSEError || error instanceof TypeError)) {
            throw error;
        }
        throw new CallError("UNAUTHORIZED", { message: "invalid assertion", cause: error });
    }
}

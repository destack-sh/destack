import type { Crypto, Environment } from "@destack/environment";
import type { DatabaseConnection } from "@destack/db";
import { IdentityError, Jws, Jwt, PublicKey } from "@destack/identity";
import type { PackageId } from "@destack/package";
import { Digest, schema, Uuid } from "@destack/schema";
import type { Subject } from "@destack/sync";
import {
    type Authentication,
    Caller,
    CLOCK_TOLERANCE_MILLISECONDS,
    KEY_ASSURANCE,
} from "../context/index.ts";
import { principal } from "../declare/index.ts";
import { AccessError } from "../error/index.ts";
import { Bearer } from "./bearer.ts";
import { Nonce } from "./nonce.ts";
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

/** Key-signed assertions principals exchange at a token endpoint (RFC 7523 jwt-bearer). */
export const Assertion = {
    /** Report whether a bearer token is a key-signed assertion, by its JWT type. */
    is(token: string): boolean {
        try {
            return Jws.header(token).typ === ASSERTION_TYPE;
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
        environment: Pick<Environment, "clock" | "random" | "crypto">,
        subject: string = key.subject.id,
    ): Promise<string> {
        // claim the key's principal for the endpoint, living a minute, under a fresh identifier
        const [algorithm] = PublicKey.algorithms(key.publicKey);
        if (algorithm === undefined) {
            throw AccessError.of("KEYS_INSECURE", "the assertion key has no signing algorithm");
        }
        const issuedAt = Math.floor(now / 1000);
        const claims = {
            jti: Uuid.v7(environment),
            iss: key.subject.id,
            sub: subject,
            aud: endpoint,
            iat: issuedAt,
            exp: issuedAt + SIGNED_LIFETIME_MILLISECONDS / 1000,
        };
        const header = { alg: algorithm, typ: ASSERTION_TYPE, kid: key.id };

        return Jwt.sign(claims, header, privateKey, environment.crypto);
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
    async verify(assertion: string, now: number): Promise<Authentication> {
        // find the standing key of the claimed issuer the header refers to
        const { kid } = refused(() => Jws.header(assertion));
        const { iss } = refused(() => Jwt.claims(assertion));
        const key =
            kid === undefined || iss === undefined
                ? undefined
                : await this.options.keys(kid, iss, now);
        if (key === undefined) {
            throw AccessError.of("ASSERTION_KEY_UNKNOWN", "assertion has no standing key");
        }

        // verify the signature, the type, and the claims of the key's principal and this endpoint
        const { crypto } = this.options;
        const verifying = Jwt.verify(
            assertion,
            await PublicKey.import(key.publicKey, crypto),
            {
                algorithms: PublicKey.algorithms(key.publicKey),
                typ: ASSERTION_TYPE,
                issuer: key.subject.id,
                audience: this.options.endpoint,
                requiredClaims: ["iat", "exp", "jti"],
                maxTokenAge: ASSERTION_LIFETIME_MILLISECONDS / 1000,
                clockTolerance: CLOCK_TOLERANCE_MILLISECONDS / 1000,
                now,
            },
            crypto,
        );
        const { claims } = await verifying.catch((error: unknown) => {
            throw IdentityError.is(error, "TOKEN_EXPIRED")
                ? AccessError.of("ASSERTION_EXPIRED", "assertion expired", { cause: error })
                : Bearer.refusal(error, "ASSERTION_INVALID", "invalid assertion");
        });
        const { sub, iat, exp, jti } = claims;
        if (sub === undefined || iat === undefined || exp === undefined || jti === undefined) {
            throw AccessError.of("ASSERTION_INVALID", "invalid assertion");
        } else if ((exp - iat) * 1000 > ASSERTION_LIFETIME_MILLISECONDS) {
            throw AccessError.of("ASSERTION_TOO_LONG", "assertion lifetime exceeds five minutes");
        }

        // spend it once under its key, refusing one spent before, and answer the key's principal
        const expiresAt = exp * 1000;
        const digest = await Digest.of(JSON.stringify([key.id, jti]), this.options.crypto);
        await new Nonce(digest, expiresAt).spend(this.options.database, now);

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
    /** The database spending each assertion's digest once until it expires. */
    readonly database: DatabaseConnection;
    /** The WebCrypto operations it verifies with. */
    readonly crypto: Crypto;
}

/** Describe an assertion's principal as the caller it authenticated until it expires. */
function asserted(
    key: AssertionKey,
    sub: string,
    audience: PackageId,
    now: number,
    expiresAt: number,
): Authentication {
    // take the key's machine, space or installation, or an installation its space signs for
    const subject = assertedSubject(key, sub);
    const isMachine = principal.machine.is(subject);
    const isSpace = principal.space.is(subject);

    return {
        caller: Caller.of(subject),
        credential: {
            kind: isMachine ? MACHINE_KEY : isSpace ? SPACE_KEY : INSTALLATION_KEY,
            id: key.id,
            audience,
            // bind an installation to its space, while a machine or a space acts as itself
            ...(isMachine || isSpace ? {} : { scope: subject.scope }),
            verifiedAt: now,
            expiresAt,
            assurance: { level: KEY_ASSURANCE, authenticatedAt: now },
        },
    };
}

/** Read the principal an assertion's subject claim asserts under a key, refusing another. */
function assertedSubject(key: AssertionKey, sub: string): Subject {
    // take the key's machine, space or installation
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
    throw AccessError.of(
        "ASSERTION_OUTSIDE_AUTHORITY",
        "assertion subject is outside its key's authority",
    );
}

/** Read an assertion's header, refusing one that is not a compact JWS. */
function refused<Value>(read: () => Value): Value {
    try {
        return read();
    } catch (error) {
        if (!IdentityError.is(error, "TOKEN_MALFORMED")) {
            throw error;
        }
        throw AccessError.of("ASSERTION_INVALID", "invalid assertion", { cause: error });
    }
}

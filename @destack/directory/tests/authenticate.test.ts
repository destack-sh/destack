import { principal } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { IdentityOperation, LocalKeyring } from "@destack/identity";
import { PackageId } from "@destack/package";
import { Identifier, type JsonObject, type JsonValue } from "@destack/schema";
import { Authentication, TokenIssuer } from "@destack/service/authentication";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { SignJWT } from "jose";
import { directoryTables, DirectoryStore, identityKey, IdentityKeystore } from "../src/index.ts";
import { keyPair } from "../src/test/index.ts";

/** The universe's issuer, which its tokens name. */
const ISSUER = "https://universe.test/";

/** The package receiving the installation's calls. */
const AUDIENCE = PackageId.parse("package-019f7480-0000-7000-8000-00000000e003");

test("verify a token by its issuer's keys, a space's in a space or outside one and the universe's, and refuse other keys, audiences, scopes, issuers and spaces without an identity", async () => {
    // start the universe's identity and the calling space's on its machine, leaving another space without one
    const storage = await TestDatabase.create("sqlite", [...directoryTables, identityKey], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const directory = new DirectoryStore(storage.database);
    const keystore = new IdentityKeystore(
        await LocalKeyring.read(LocalKeyring.generate()),
        directory,
    );
    await keystore.generate(storage.database, Scope.universe.id);
    const home = Identifier.create("space");
    const target = Identifier.create("space");
    const placement = {
        id: home,
        scope: Identifier.create("account"),
        machine: Identifier.create("machine"),
        epoch: 1,
    };
    await directory.place(placement);
    const signing = await keyPair();
    const rotation = await keyPair();
    const intruder = await keyPair();
    await directory.apply(
        await IdentityOperation.sign(
            {
                subject: home,
                previous: null,
                signingKeys: [signing.key],
                rotationKeys: [rotation.key],
            },
            rotation.privateKey,
        ),
        placement,
    );

    // sign an installation's calls with the space's key or another and verify them
    const installation = principal.installation.reference(home, Identifier.create("installation"));
    const anonymous = principal.installation.reference(
        Identifier.create("space"),
        Identifier.create("installation"),
    );
    const sign = (
        scope: string,
        options: { key?: CryptoKey; audience?: PackageId; caller?: typeof installation } = {},
    ) =>
        signInstallation(
            options.key ?? signing.privateKey,
            options.caller ?? installation,
            options.audience ?? AUDIENCE,
            scope,
        );
    const verify = async (request: Request, scope?: string) => {
        const verified = directory.authenticate(request, {
            audience: AUDIENCE,
            universe: ISSUER,
            ...(scope === undefined ? {} : { scope }),
        });
        const refused = await refusal(verified);
        if (refused !== "done") {
            return refused.join(": ");
        }
        const { claims } = await verified;

        return [claims.subject.id, claims.scope];
    };
    const other = PackageId.parse("package-019f7480-0000-7000-8000-00000000e004");
    const owner = principal.user.reference(Scope.universe.id, Identifier.create("user"));
    const user = bearer(await signUser(keystore, storage.database, owner, target));
    const stranger = bearer(fakeToken({ iss: "https://universe.test" }));
    expect([
        await verify(await sign(target), target),
        await verify(await sign(target, { key: intruder.privateKey }), target),
        await verify(await sign(target, { audience: other }), target),
        await verify(await sign(home), target),
        await verify(await sign(target, { caller: anonymous }), target),
        await verify(await sign(home)),
        await verify(await sign(target)),
        await verify(user, target),
        await verify(stranger, target),
    ]).toEqual([
        [installation.id, target],
        "UNAUTHORIZED: invalid access token",
        "UNAUTHORIZED: invalid access token",
        "UNAUTHORIZED: invalid access token claims",
        `UNAUTHORIZED: ${anonymous.scope} has no identity`,
        [installation.id, home],
        [installation.id, target],
        [owner.id, target],
        "UNAUTHORIZED: invalid access token",
    ]);

    // accept tokens of spaces alone without the universe's issuer, refusing the universe's
    const spacesAlone = async (request: Request) => {
        const refused = await refusal(
            directory.authenticate(request, { audience: AUDIENCE, scope: target }),
        );

        return refused === "done" ? "accepted" : refused.join(": ");
    };
    expect([await spacesAlone(await sign(target)), await spacesAlone(user)]).toEqual([
        "accepted",
        "UNAUTHORIZED: invalid access token",
    ]);
});

/** Sign an installation's call to a package in a space with a key, as its space signs. */
async function signInstallation(
    key: CryptoKey,
    installation: ReturnType<typeof principal.installation.reference>,
    audience: PackageId,
    scope: string,
): Promise<Request> {
    const now = Date.now();
    const caller = new Authentication({
        credential: { kind: "installation", id: Identifier.create("instance") },
        audience,
        scope,
        subject: installation,
        subjects: [installation],
        deployments: [{ subject: installation, id: Identifier.create("deployment") }],
        verifiedAt: now,
        expiresAt: now + 60_000,
    });
    const issuer = new TokenIssuer({
        authority: { kind: "space", space: installation.scope },
        issuer: installation.scope,
        sign: (claims) => new SignJWT(claims).setProtectedHeader({ alg: "ES256" }).sign(key),
    });
    const { accessToken } = await issuer.issue(caller);

    return new Request("https://notes.test/.destack/service", {
        headers: { authorization: `Bearer ${accessToken}` },
    });
}

/** Sign a universe token for a user calling a package in a space, as the account service signs it. */
async function signUser(
    keystore: IdentityKeystore,
    database: DatabaseConnection,
    user: ReturnType<typeof principal.user.reference>,
    scope: string,
): Promise<string> {
    const now = Date.now();
    const caller = new Authentication({
        credential: { kind: "session", id: Identifier.create("session") },
        audience: AUDIENCE,
        scope,
        subject: user,
        subjects: [user],
        verifiedAt: now,
        expiresAt: now + 60_000,
    });
    const issuer = new TokenIssuer({
        authority: { kind: "universe" },
        issuer: ISSUER,
        sign: (claims) => keystore.sign(database, Scope.universe.id, claims),
    });

    return (await issuer.issue(caller)).accessToken;
}

/** Send a token as a request's bearer credential. */
function bearer(token: string): Request {
    return new Request("https://notes.test", { headers: { authorization: `Bearer ${token}` } });
}

/** Encode an unsigned token with some claims, as a token of an unknown issuer looks before verification. */
function fakeToken(claims: JsonObject): string {
    return `${encodeSegment({ alg: "ES256" })}.${encodeSegment({ ...claims, caller: {} })}.signature`;
}

/** Encode a token segment as base64url JSON. */
function encodeSegment(value: JsonValue): string {
    return new TextEncoder()
        .encode(JSON.stringify(value))
        .toBase64({ alphabet: "base64url", omitPadding: true });
}

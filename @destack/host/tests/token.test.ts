import { principal } from "@destack/access";
import { keyPair } from "@destack/directory/test";
import { DirectoryStore, IdentityOperation } from "@destack/directory";
import { PackageId } from "@destack/package";
import { Identifier, type JsonObject, type JsonValue } from "@destack/schema";
import { Authentication, TokenIssuer } from "@destack/service/authentication";
import { afterAll, beforeAll, expect, refusal, test } from "@destack/test";
import { SignJWT } from "jose";
import { SpaceToken } from "../src/identity/index.ts";
import { AccountFixture, ids } from "../src/test/index.ts";

/** The package receiving the installation's calls. */
const AUDIENCE = PackageId.parse("package-019f7480-0000-7000-8000-00000000e003");

/** The account service the scenario enrolls its hosts in. */
let accounts: AccountFixture;

beforeAll(async () => {
    accounts = await AccountFixture.open();
});

afterAll(async () => {
    await accounts[Symbol.asyncDispose]();
});

test("verify a token a space signed with its key, in a space or for a universe service, and refuse other keys, audiences, scopes and spaces without an identity", async () => {
    // start the calling space's identity in its cell, and leave another space without one
    const directory = new DirectoryStore(accounts.database);
    const home = Identifier.create("space");
    const target = Identifier.create("space");
    const zone = { id: home, scope: ids.account, cell: "host-1", epoch: 1 };
    await directory.place(zone);
    const signing = await keyPair();
    const rotation = await keyPair();
    const intruder = await keyPair();
    await directory.apply(
        await IdentityOperation.sign(
            { space: home, previous: null, signingKey: signing.key, rotationKeys: [rotation.key] },
            rotation.privateKey,
        ),
        zone,
    );

    // sign an installation's calls with the space's key or another, and verify them for a space or the universe
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
    const verify = async (request: Request, spaceId?: string) => {
        const verified = SpaceToken.verify(request, {
            directory,
            audience: AUDIENCE,
            ...(spaceId === undefined ? {} : { spaceId }),
        });
        const refused = await refusal(verified);
        if (refused !== "done") {
            return refused.join(": ");
        }
        const { claims } = await verified;

        return [claims.subject.id, claims.scope];
    };
    const other = PackageId.parse("package-019f7480-0000-7000-8000-00000000e004");
    const user = new Request("https://notes.test", {
        headers: { authorization: `Bearer ${fakeToken({ iss: "https://universe.test" })}` },
    });
    expect([
        await verify(await sign(target), target),
        await verify(await sign(target, { key: intruder.privateKey }), target),
        await verify(await sign(target, { audience: other }), target),
        await verify(await sign(home), target),
        await verify(await sign(target, { caller: anonymous }), target),
        await verify(await sign(home)),
        await verify(await sign(target)),
        [
            SpaceToken.accepts(await sign(target)),
            SpaceToken.accepts(user),
            SpaceToken.accepts(new Request("https://notes.test")),
        ],
    ]).toEqual([
        [installation.id, target],
        "UNAUTHORIZED: invalid access token",
        "UNAUTHORIZED: invalid access token",
        "UNAUTHORIZED: invalid access token claims",
        "UNAUTHORIZED: the token's space has no identity",
        [installation.id, home],
        "UNAUTHORIZED: invalid access token claims",
        [true, false, false],
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
        authority: { kind: "space", spaceId: installation.scope },
        issuer: installation.scope,
        sign: (claims) => new SignJWT(claims).setProtectedHeader({ alg: "ES256" }).sign(key),
    });
    const { accessToken } = await issuer.issue(caller);

    return new Request("https://notes.test/.destack/service", {
        headers: { authorization: `Bearer ${accessToken}` },
    });
}

/** Encode an unsigned token with some claims, as a user token looks before verification. */
function fakeToken(claims: JsonObject): string {
    return `${encodeSegment({ alg: "ES256" })}.${encodeSegment({ ...claims, caller: {} })}.signature`;
}

/** Encode a token segment as base64url JSON. */
function encodeSegment(value: JsonValue): string {
    return new TextEncoder()
        .encode(JSON.stringify(value))
        .toBase64({ alphabet: "base64url", omitPadding: true });
}

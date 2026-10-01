import { v7 } from "uuid";
import { principal } from "@destack/access";
import { DirectoryDatabase } from "@destack/directory";
import { PackageId } from "@destack/package";
import { identifier } from "@destack/schema";
import { Caller, TokenIssuer } from "@destack/service/authentication";
import { afterAll, beforeAll, expect, test } from "@destack/test";
import { Condition } from "@destack/db/query";
import { type HostIdentity, SpaceToken } from "../src/identity/index.ts";
import { HostKey } from "../src/object/index.ts";
import { GlobalFixture, ids } from "../src/test/index.ts";

/** The package receiving the installation's calls. */
const AUDIENCE = PackageId.parse("package-019f7480-0000-7000-8000-00000000e003");

/** The global tier the scenario enrolls its hosts in. */
let global: GlobalFixture;

beforeAll(async () => {
    global = await GlobalFixture.open();
});

afterAll(async () => {
    await global[Symbol.asyncDispose]();
});

test("verify an installation token its space's cell signed for another space, and refuse other signers, audiences and spaces", async () => {
    // enroll a cell and a stranger, and place the calling space in the cell
    const cell = await global.enroll(ids.account);
    const stranger = await global.enroll(ids.account);
    const directory = new DirectoryDatabase(global.database);
    const home = identifier("space").parse(`space-${v7()}`);
    const target = identifier("space").parse(`space-${v7()}`);
    await directory.publish(cell.hostId, ids.account, "https://cell.test");
    await directory.publish(stranger.hostId, ids.account, "https://stranger.test");
    await directory.place({ id: home, scope: ids.account, cell: cell.hostId, epoch: 1 });

    // sign a call of an installation of the home space to the target space, as a host
    const installation = principal.installation.reference(home, `installation-${v7()}`);
    const sign = (identity: typeof cell, audience = AUDIENCE, scope: string = target) =>
        signInstallation(identity, installation, audience, scope);
    const verify = (request: Request) =>
        SpaceToken.verify(request, {
            directory,
            keys: async (accountId, hostId, now) => {
                const keys = await global.host(cell).hostKey.list({
                    accountId,
                    where: Condition.all(
                        Condition.eq("parentId", hostId),
                        HostKey.authenticates(now),
                    ),
                });

                return keys.items;
            },
            audience: AUDIENCE,
            spaceId: target,
        }).then(
            (caller) => [caller.authentication.subject.id, caller.authentication.scope],
            (error: { code: string; message: string }) => `${error.code}: ${error.message}`,
        );

    // accept the cell's token for the target space, and refuse a stranger's, another audience's and another space's
    const other = PackageId.parse("package-019f7480-0000-7000-8000-00000000e004");
    expect([
        await verify(await sign(cell)),
        await verify(await sign(stranger)),
        await verify(await sign(cell, other)),
        await verify(await sign(cell, AUDIENCE, home)),
    ]).toEqual([
        [installation.id, target],
        "UNAUTHORIZED: the token's signer serves no space of its installation",
        "UNAUTHORIZED: invalid access token",
        "UNAUTHORIZED: invalid access token claims",
    ]);
});

test("verify an installation token to a universe service against stored keys, scoped to its own space, and recognize space tokens by their issuer", async () => {
    // enroll a cell and place the calling space in it
    const cell = await global.enroll(ids.account);
    const directory = new DirectoryDatabase(global.database);
    const home = identifier("space").parse(`space-${v7()}`);
    await directory.publish(cell.hostId, ids.account, "https://cell.test");
    await directory.place({ id: home, scope: ids.account, cell: cell.hostId, epoch: 1 });
    const installation = principal.installation.reference(home, `installation-${v7()}`);
    const verify = (request: Request) =>
        SpaceToken.verify(request, {
            directory,
            keys: (accountId, hostId, now) =>
                HostKey.authenticating(global.database, accountId, hostId, now),
            audience: AUDIENCE,
        }).then(
            (caller) => [caller.authentication.subject.id, caller.authentication.scope],
            (error: { code: string; message: string }) => `${error.code}: ${error.message}`,
        );

    // accept a token scoped to the installation's own space, refuse one scoped to another space
    const own = await signInstallation(cell, installation, AUDIENCE, home);
    const other = await signInstallation(cell, installation, AUDIENCE, `space-${v7()}`);
    const user = new Request("https://notes.test", {
        headers: { authorization: `Bearer ${await fakeToken({ iss: "https://universe.test" })}` },
    });
    expect([
        await verify(own),
        await verify(other),
        [
            SpaceToken.accepts(own),
            SpaceToken.accepts(user),
            SpaceToken.accepts(new Request("https://notes.test")),
        ],
    ]).toEqual([
        [installation.id, home],
        "UNAUTHORIZED: invalid access token claims",
        [true, false, false],
    ]);
});

/** Sign an installation's call to a package in a space, as the cell of the installation's space. */
async function signInstallation(
    identity: HostIdentity,
    installation: ReturnType<typeof principal.installation.reference>,
    audience: PackageId,
    scope: string,
): Promise<Request> {
    const now = Date.now();
    const caller = new Caller({
        credential: { kind: "installation", id: `instance-${v7()}` },
        audience,
        scope,
        subject: installation,
        subjects: [installation],
        deployments: [
            { subject: installation, id: identifier("deployment").parse(`deployment-${v7()}`) },
        ],
        verifiedAt: now,
        expiresAt: now + 60_000,
    });
    const issuer = new TokenIssuer({
        authority: { kind: "space", spaceId: installation.scope },
        issuer: identity.hostId,
        sign: (claims) => identity.signToken(claims),
    });
    const { accessToken } = await issuer.issue(caller);

    return new Request("https://notes.test/.destack/service", {
        headers: { authorization: `Bearer ${accessToken}` },
    });
}

/** Encode an unsigned token carrying claims, as a user token looks before verification. */
async function fakeToken(claims: Readonly<Record<string, unknown>>): Promise<string> {
    const encode = (value: unknown) =>
        new TextEncoder()
            .encode(JSON.stringify(value))
            .toBase64({ alphabet: "base64url", omitPadding: true });

    return `${encode({ alg: "ES256" })}.${encode({ ...claims, caller: {} })}.signature`;
}

import type { Restriction } from "@destack/access";
import { Subject } from "@destack/sync";
import { PackageId } from "@destack/package";
import { RequestId } from "@destack/service/request";
import { expect, test } from "@destack/test";
import { v7 } from "uuid";
import { account } from "../src/object/account.ts";
import { serviceAccount } from "../src/object/service.ts";
import { Credential } from "../src/object/credential.ts";
import { AccountFixture, claims, identity } from "./fixture.ts";

/** A package another service declares with permissions for restrictions. */
const VAULT = PackageId.parse(`package-${v7()}`);

/** One day in milliseconds. */
const DAY = 24 * 60 * 60 * 1000;

/** Issue a personal token from a client-created secret, authenticate within its restrictions, and revoke it. */
test("issue, use and revoke personal access tokens", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const { accountId, spaceId, handle } = await fixture.createSpace(owner);

    // restrict a token to reading the account and one space's secrets, from a client-made secret
    const credential = await Credential.create("personal-access-token");
    const restrictions: Restriction[] = [
        { ...account.permission("read"), scope: accountId },
        { packageId: VAULT, type: "version", name: "read", scope: spaceId },
    ];
    const issuing = {
        userId: owner.id,
        requestId: RequestId.create(),
        name: "agent",
        digest: credential.digest,
        restrictions,
        expiresAt: Date.now() + DAY,
    };

    // challenge a single factor, then issue after the owner authenticates again
    await expect(owner.client.personalAccessToken.issue(issuing)).rejects.toMatchObject({
        code: "INSUFFICIENT_AUTHENTICATION",
        status: 401,
        message: "authenticate again at the required assurance",
        data: { assurance: 2, maxAge: 15 * 60 * 1000 },
    });
    await fixture.elevate(owner);
    const issued = await owner.client.personalAccessToken.issue(issuing);
    expect(issued).toEqual({
        id: issued.id,
        createdAt: issued.createdAt,
        createdBy: Subject.key(owner.subject),
        updatedBy: Subject.key(owner.subject),
        updatedAt: issued.createdAt,
        revision: 1,
        tags: {},
        scope: owner.id,
        name: "agent",
        restrictions,
        expiresAt: issued.expiresAt,
        revokedAt: null,
    });
    expect(credential.secret.startsWith("dst_pat_")).toBe(true);

    // authenticate as the user through the token, carrying its restrictions into exchanged access tokens
    const bearer = fixture.connect(() => ({ authorization: `Bearer ${credential.secret}` }));
    expect(identity(await bearer.authentication.current())).toEqual({
        subject: owner.subject,
        credential: { kind: "personal-access-token", id: issued.id },
        profile: { subject: owner.subject, name: "owner@example.com", handle },
    });
    const exchanged = await bearer.authentication.exchange({ audience: VAULT, spaceId });
    expect(claims(exchanged.accessToken).permissions).toEqual([
        { ...account.permission("read"), scope: spaceId },
        { packageId: VAULT, type: "version", name: "read", scope: spaceId },
    ]);

    // refuse unknown account permissions, unknown scopes and unbounded expiries
    const refusals = await Promise.all(
        [
            { restrictions: [{ ...account.permission("read"), name: "fly", scope: accountId }] },
            {
                restrictions: [
                    { packageId: VAULT, type: "version", name: "read", scope: "elsewhere" },
                ],
            },
            { expiresAt: Date.now() - 1 },
            { expiresAt: Date.now() + 400 * DAY },
        ].map((change) =>
            owner.client.personalAccessToken
                .issue({
                    userId: owner.id,
                    requestId: RequestId.create(),
                    name: "refused",
                    digest: credential.digest,
                    restrictions,
                    expiresAt: Date.now() + DAY,
                    ...change,
                })
                .then(
                    () => "issued",
                    (error: { code: string; message: string }) => [error.code, error.message],
                ),
        ),
    );
    expect(refusals).toEqual([
        ["BAD_REQUEST", "unknown permission account.fly"],
        ["BAD_REQUEST", "token cannot act in elsewhere"],
        ["BAD_REQUEST", "token expiry must lie within 366 days from now"],
        ["BAD_REQUEST", "token expiry must lie within 366 days from now"],
    ]);

    // hide the token from another user, who cannot issue tokens for the owner either
    const other = await fixture.signIn("other@example.com");
    await expect(other.client.personalAccessToken.list({ userId: owner.id })).rejects.toMatchObject(
        { code: "NOT_FOUND", message: `no scope ${owner.id}` },
    );
    await expect(
        other.client.personalAccessToken.issue({
            userId: owner.id,
            requestId: RequestId.create(),
            name: "stolen",
            digest: (await Credential.create("personal-access-token")).digest,
            restrictions: [],
            expiresAt: Date.now() + DAY,
        }),
    ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no scope ${owner.id}` });

    // revoke the token once to stop it authenticating at once
    const revoked = await owner.client.personalAccessToken.revoke({
        userId: owner.id,
        id: issued.id,
        requestId: RequestId.create(),
    });
    expect(revoked.revokedAt).toBe(revoked.updatedAt);
    await expect(bearer.authentication.current()).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "invalid personal access token",
    });
    await expect(
        owner.client.personalAccessToken.revoke({
            userId: owner.id,
            id: issued.id,
            requestId: RequestId.create(),
        }),
    ).rejects.toMatchObject({ code: "CONFLICT", message: "personal-access-token is revoked" });
});

/** Issue service tokens acting as their service account in its account's spaces, ending with the service account. */
test("issue and use service tokens until their service account is revoked", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const { accountId, spaceId } = await fixture.createSpace(owner);
    const elsewhere = await fixture.createSpace(owner);

    // issue a token of a service account restricted to one of the account's spaces
    const automation = await owner.client.serviceAccount.create({
        accountId,
        requestId: RequestId.create(),
        name: "automation",
    });
    const credential = await Credential.create("service-token");
    const restrictions = [{ packageId: VAULT, type: "version", name: "read", scope: spaceId }];
    const issued = await owner.client.serviceToken.issue({
        accountId,
        parentId: automation.id,
        requestId: RequestId.create(),
        name: "deploy",
        digest: credential.digest,
        restrictions,
        expiresAt: Date.now() + DAY,
    });

    // authenticate as the service account within the restrictions
    const software = serviceAccount.reference(accountId, automation.id);
    const bearer = fixture.connect(() => ({ authorization: `Bearer ${credential.secret}` }));
    expect(identity(await bearer.authentication.current())).toEqual({
        subject: software,
        credential: { kind: "service-token", id: issued.id },
        profile: { subject: software, name: "automation", handle: null },
    });
    const exchanged = await bearer.authentication.exchange({ audience: VAULT, spaceId });
    expect(claims(exchanged.accessToken).permissions).toEqual(restrictions);

    // refuse restrictions in another account's spaces, and exchanges for them
    await expect(
        owner.client.serviceToken.issue({
            accountId,
            parentId: automation.id,
            requestId: RequestId.create(),
            name: "outside",
            digest: (await Credential.create("service-token")).digest,
            restrictions: [{ ...restrictions[0]!, scope: elsewhere.spaceId }],
            expiresAt: Date.now() + DAY,
        }),
    ).rejects.toMatchObject({
        code: "BAD_REQUEST",
        message: `token cannot act in ${elsewhere.spaceId}`,
    });
    await expect(
        bearer.authentication.exchange({ audience: VAULT, spaceId: elsewhere.spaceId }),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "service token belongs to another account",
    });

    // end every token of the revoked service account and issue no more
    await owner.client.serviceAccount.revoke({
        accountId,
        id: automation.id,
        requestId: RequestId.create(),
    });
    await expect(bearer.authentication.current()).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "invalid service token",
    });
    await expect(
        owner.client.serviceToken.issue({
            accountId,
            parentId: automation.id,
            requestId: RequestId.create(),
            name: "late",
            digest: (await Credential.create("service-token")).digest,
            restrictions: [],
            expiresAt: Date.now() + DAY,
        }),
    ).rejects.toMatchObject({ code: "CONFLICT", message: "service account is revoked" });
});

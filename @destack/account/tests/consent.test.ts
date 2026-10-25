import objectPackage from "@destack/object/package";
import { principal } from "@destack/access";
import { eq } from "@destack/db";
import { Identifier, present } from "@destack/schema";
import { RequestId, ObjectReference, Scope } from "@destack/sync";
import { expect, refusal, single, test } from "@destack/test";
import { client, session } from "../src/object/index.ts";
import accountPackage from "../src/package.ts";
import { TestAccountStore, TestJwt, OWNER_EMAIL } from "../src/test/index.ts";

test("let an installation act for a person under the grant the person created, until they revoke it", async () => {
    // store the owner's browser with a push subscription, and an installation of their space with an identity
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { spaceId, machine } = await fixture.createSpace(owner);
    await fixture.startIdentity(spaceId, machine);
    const browser = await owner.client(accountPackage).client.create({
        userId: owner.id,
        requestId: RequestId.create(fixture.store.environment),
        name: "Firefox",
        kind: "browser",
    });
    await owner.client(accountPackage).client.subscribe({
        userId: owner.id,
        id: browser.id,
        requestId: RequestId.create(fixture.store.environment),
        push: { url: "https://push.example.com/1", keys: { p256dh: "BNcR", auth: "tBHI" } },
    });
    const inbox = principal.installation.reference(
        spaceId,
        Identifier.create("installation", fixture.environment),
    );

    // consent as the owner: grant the installation forgetting push subscriptions
    const granted = await owner.client(objectPackage).grant.create({
        scope: owner.id,
        requestId: RequestId.create(fixture.store.environment),
        delegate: inbox,
        permissions: [{ ...client.permission("unsubscribe"), scope: owner.id }],
    });

    // exchange the installation's token for the owner's under a named or the only grant
    const exchange = async (grants?: readonly Identifier<"grant">[]) =>
        fixture.source(inbox).exchange(accountPackage.id, {
            subject: owner.subject,
            ...(grants === undefined ? {} : { grants }),
        });
    const acting = fixture.issuer.accounts((await exchange([granted.id])).accessToken);

    // refuse a chain linking a grant the universe stores no record of
    const unknown = Identifier.create("grant", fixture.environment);
    const unlinked = await refusal(exchange([granted.id, unknown]));

    // forget the push subscription for the owner
    await acting.client.unsubscribe({
        userId: owner.id,
        id: browser.id,
        requestId: RequestId.create(fixture.store.environment),
    });
    const forgotten = await fixture.storage.database
        .select({ push: client.table.push })
        .from(client.table)
        .where(eq(client.table.id, browser.id));

    // revoke the grant, refusing any new exchange
    const listed = await owner.client(objectPackage).grant.list({ scope: owner.id });
    const grant = present(listed.items[0], "the owner's grant");
    await owner.client(objectPackage).grant.revoke({
        scope: owner.id,
        id: grant.id,
        requestId: RequestId.create(fixture.store.environment),
        revision: grant.revision,
    });
    expect({
        listed: listed.items.map((item) => [
            item.id,
            ObjectReference.at("delegate", item),
            item.permissions,
        ]),
        forgotten,
        unlinked,
        exchanged: await refusal(exchange()),
    }).toEqual({
        listed: [[granted.id, inbox, [{ ...client.permission("unsubscribe"), scope: owner.id }]]],
        forgotten: [{ push: null }],
        unlinked: [
            "BAD_REQUEST",
            "INVALID_GRANT",
            `the token endpoint refused the request: invalid_grant: grant ${unknown} is revoked`,
        ],
        exchanged: [
            "BAD_REQUEST",
            "INVALID_GRANT",
            `the token endpoint refused the request: invalid_grant: ${owner.id} has no grant to ${inbox.id}`,
        ],
    });
});

test("stamp a token acting for a person under their grant with the universe's position, living within the person's session", async () => {
    // grant an installation of the owner's space reading their clients
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { spaceId, machine } = await fixture.createSpace(owner);
    await fixture.startIdentity(spaceId, machine);
    const inbox = principal.installation.reference(
        spaceId,
        Identifier.create("installation", fixture.environment),
    );
    await owner.client(objectPackage).grant.create({
        scope: owner.id,
        requestId: RequestId.create(fixture.store.environment),
        delegate: inbox,
        permissions: [{ ...client.permission("read"), scope: owner.id }],
    });

    // exchange the installation's token for the owner's within their session
    const [signedIn] = await fixture.storage.database
        .select({ id: session.table.id })
        .from(session.table)
        .where(eq(session.table.userId, owner.id));
    const sessionId = present(signedIn, "the owner's session").id;
    const exchange = async () =>
        fixture.source(inbox).exchange(accountPackage.id, {
            subject: owner.subject,
            session: sessionId,
        });
    const { log } = fixture.storage.database;
    const granted = await log.position();
    const { accessToken } = await exchange();
    const exchanged = await log.position();
    const stamped = single(
        present(TestJwt.claims(accessToken).caller.credential.watermarks, "the watermarks"),
    );

    // end the session, refusing the next exchange within it
    await owner.client(accountPackage).session.revoke({
        userId: owner.id,
        id: sessionId,
        requestId: RequestId.create(fixture.store.environment),
    });

    // stamp the universe's position between the grant's write and the exchange
    const { sequence, ...position } = stamped;
    const { sequence: grantedAt, ...grantedPosition } = granted;
    expect({ position, ended: await refusal(exchange()) }).toEqual({
        position: { ...grantedPosition, scope: Scope.universe.id },
        ended: [
            "BAD_REQUEST",
            "INVALID_GRANT",
            "the token endpoint refused the request: invalid_grant: the person's session ended",
        ],
    });
    expect(sequence).toBeGreaterThanOrEqual(grantedAt);
    expect(sequence).toBeLessThanOrEqual(exchanged.sequence);
});

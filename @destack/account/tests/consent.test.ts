import { principal } from "@destack/access";
import { eq } from "@destack/db";
import { Identifier, present, schema } from "@destack/schema";
import { decodeJwt } from "jose";
import { RequestId } from "@destack/runtime/request";
import { ObjectReference, Scope, Subject, Watermark } from "@destack/sync";
import { expect, test } from "@destack/test";
import { client, session } from "../src/object/index.ts";
import { ACCESS_TOKEN_TYPE } from "../src/oauth/index.ts";
import accountPackage from "../src/package.ts";
import { AccountFixture, outcome } from "../src/test/index.ts";

test("let an installation act for a person exactly as the grant the person's own client created allows, until they revoke it", async () => {
    // store the owner's browser with a push subscription, and an installation of their space with an identity
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { spaceId, machine } = await fixture.createSpace(owner);
    await fixture.startIdentity(spaceId, machine);
    const browser = await owner.client.client.create({
        userId: owner.id,
        requestId: RequestId.create(Date.now()),
        name: "Firefox",
        kind: "browser",
    });
    await owner.client.client.subscribe({
        userId: owner.id,
        id: browser.id,
        requestId: RequestId.create(Date.now()),
        push: { url: "https://push.example.com/1", keys: { p256dh: "BNcR", auth: "tBHI" } },
    });
    const inbox = principal.installation.reference(spaceId, Identifier.create("installation"));

    // consent as the owner: grant the installation forgetting push subscriptions
    const granted = await owner.client.grant.create({
        userId: owner.id,
        requestId: RequestId.create(Date.now()),
        delegate: inbox,
        permissions: [{ ...client.permission("unsubscribe"), scope: owner.id }],
    });

    // exchange the installation's own token for the owner's under a named or the only grant
    const exchange = async (grants?: readonly string[]) =>
        fixture.exchangeToken(await fixture.installationToken(inbox), ACCESS_TOKEN_TYPE, {
            audience: accountPackage.id,
            subject: Subject.key(owner.subject),
            ...(grants === undefined ? {} : { grants: grants.join(" ") }),
        });
    const acting = fixture.bearer((await exchange([granted.id])).accessToken);

    // refuse a chain linking a grant the universe keeps no record of
    const unknown = Identifier.create("grant");
    const unlinked = await outcome(exchange([granted.id, unknown]));

    // forget the push subscription for the owner, and refuse revoking the client the grant leaves out
    await acting.client.unsubscribe({
        userId: owner.id,
        id: browser.id,
        requestId: RequestId.create(Date.now()),
    });
    const [forgotten] = await fixture.storage.database
        .select({ push: client.table.push })
        .from(client.table)
        .where(eq(client.table.id, browser.id));
    const ungranted = await outcome(
        acting.client.revoke({
            userId: owner.id,
            id: browser.id,
            requestId: RequestId.create(Date.now()),
        }),
    );

    // revoke the grant, refusing the token it was exchanged for and any new exchange
    const listed = await owner.client.grant.list({ userId: owner.id });
    const grant = present(listed.items[0], "the owner's grant");
    await owner.client.grant.revoke({
        userId: owner.id,
        id: grant.id,
        requestId: RequestId.create(Date.now()),
        revision: grant.revision,
    });
    const revoked = await outcome(
        acting.client.unsubscribe({
            userId: owner.id,
            id: browser.id,
            requestId: RequestId.create(Date.now()),
        }),
    );
    expect({
        listed: listed.items.map((item) => [
            item.id,
            ObjectReference.at("delegate", item),
            item.permissions,
        ]),
        push: forgotten?.push,
        unlinked,
        ungranted,
        revoked,
        exchanged: await outcome(exchange()),
    }).toEqual({
        listed: [[granted.id, inbox, [{ ...client.permission("unsubscribe"), scope: owner.id }]]],
        push: null,
        unlinked: `BAD_REQUEST: invalid_grant: grant ${unknown} is revoked`,
        ungranted: `INSUFFICIENT_GRANT: grant ${granted.id} does not give revoke on client ${browser.id}`,
        revoked: `UNAUTHORIZED: grant ${granted.id} is revoked`,
        exchanged: `BAD_REQUEST: invalid_grant: ${owner.id} has no grant to ${inbox.id}`,
    });
});

test("stamp a token acting for a person under their grant with the universe's position, living within the person's session", async () => {
    // grant an installation of the owner's space reading their clients
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { spaceId, machine } = await fixture.createSpace(owner);
    await fixture.startIdentity(spaceId, machine);
    const inbox = principal.installation.reference(spaceId, Identifier.create("installation"));
    await owner.client.grant.create({
        userId: owner.id,
        requestId: RequestId.create(Date.now()),
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
        fixture.exchangeToken(await fixture.installationToken(inbox), ACCESS_TOKEN_TYPE, {
            audience: accountPackage.id,
            subject: Subject.key(owner.subject),
            session: sessionId,
        });
    const { log } = fixture.storage.database;
    const granted = await log.position();
    const { accessToken } = await exchange();
    const exchanged = await log.position();
    const caller = schema
        .looseObject({
            credential: schema.looseObject({ watermarks: schema.array(Watermark) }),
        })
        .parse(decodeJwt(accessToken)["caller"]);
    const [stamped] = caller.credential.watermarks;

    // end the session, refusing the next exchange within it
    await owner.client.session.revoke({
        userId: owner.id,
        id: sessionId,
        requestId: RequestId.create(Date.now()),
    });
    // stamp one universe position between the grant's write and the exchange
    expect({
        stamps: caller.credential.watermarks.length,
        stamped: [stamped?.scope, stamped?.epoch],
        isFresh:
            stamped !== undefined &&
            stamped.sequence >= granted.sequence &&
            stamped.sequence <= exchanged.sequence,
        ended: await outcome(exchange()),
    }).toEqual({
        stamps: 1,
        stamped: [Scope.universe.id, granted.epoch],
        isFresh: true,
        ended: "BAD_REQUEST: invalid_grant: the person's session ended",
    });
});

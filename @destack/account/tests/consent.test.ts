import { principal } from "@destack/access";
import { eq } from "@destack/db";
import { Identifier, present } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { ObjectReference, Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { client } from "../src/object/index.ts";
import { ACCESS_TOKEN_TYPE } from "../src/oauth/index.ts";
import accountPackage from "../src/package.ts";
import { AccountFixture, outcome } from "../src/test/index.ts";

test("let an installation act for a person exactly as the grant the person's own client created allows, until they revoke it", async () => {
    // keep the owner's browser with a push subscription, and an installation of their space with an identity
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

    // exchange the installation's own token for the owner's under the grant, outside any space
    const exchange = async () =>
        fixture.exchangeToken(await fixture.installationToken(inbox), ACCESS_TOKEN_TYPE, {
            audience: accountPackage.id,
            subject: Subject.key(owner.subject),
        });
    const acting = fixture.bearer((await exchange()).accessToken);

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
        ungranted,
        revoked,
        exchanged: await outcome(exchange()),
    }).toEqual({
        listed: [[granted.id, inbox, [{ ...client.permission("unsubscribe"), scope: owner.id }]]],
        push: null,
        ungranted: `INSUFFICIENT_GRANT: grant ${granted.id} does not give revoke on client ${browser.id}`,
        revoked: `UNAUTHORIZED: grant ${granted.id} is revoked`,
        exchanged: `BAD_REQUEST: invalid_grant: ${owner.id} has no grant to ${inbox.id}`,
    });
});

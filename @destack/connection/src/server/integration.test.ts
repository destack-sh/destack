import { Identifier, schema } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import type { ObjectReference } from "@destack/sync";
import { DEVICE_CODE } from "../oauth/server.ts";
import { ConnectionFixture } from "../test/index.ts";

/** A space of another account, outside the home's. */
const OUTSIDE = Identifier.create("space");

/** Ask an integration's home for a device authorization, token and revocation as a space. */
async function ask(home: ConnectionFixture, integration: ObjectReference, space: string) {
    // call the home as the space
    const client = home.home(space).home(integration.scope);
    const target = {
        scope: integration.scope,
        id: schema.identifier("integration").parse(integration.id),
    };

    // start a device authorization, approve it, then request its token and revoke it
    const device = await client.integration.authorizeDevice({ ...target, request: { scopes: [] } });
    home.server.approveDevice(device.userCode);
    const token = await client.integration.requestToken({
        ...target,
        grant: { grant_type: DEVICE_CODE, device_code: device.deviceCode },
    });
    await client.integration.revokeToken({
        ...target,
        revocation: { token: token.accessToken, hint: "access_token" },
    });

    return { userCode: device.userCode, token: token.accessToken };
}

test("refuse a space outside a private integration's scope its tokens, device authorizations and revocations, and serve every space a public one's", async () => {
    // keep a private and a public integration at their home
    const home = await ConnectionFixture.open();
    onTestFinished(() => home.close());
    const owned = await home.staticIntegration();
    const shared = await home.staticIntegration({ isPublic: true });

    // call each as a space placed in another account
    const client = home.home(OUTSIDE).home(owned.scope);
    const target = { scope: owned.scope, id: schema.identifier("integration").parse(owned.id) };
    const refused = [
        await refusal(client.integration.authorizeDevice({ ...target, request: { scopes: [] } })),
        await refusal(
            client.integration.requestToken({
                ...target,
                grant: { grant_type: "refresh_token", refresh_token: "refresh-1" },
            }),
        ),
        await refusal(
            client.integration.revokeToken({
                ...target,
                revocation: { token: "access-1", hint: "access_token" },
            }),
        ),
    ];

    // hide the private integration from the outside space, and serve the public one
    const hidden = ["NOT_FOUND", `no integration ${owned.id}`];
    expect({
        refused,
        served: await ask(home, shared, OUTSIDE),
        revoked: home.server.revoked,
    }).toEqual({
        refused: [hidden, hidden, hidden],
        served: { userCode: "USER-1", token: "access-1" },
        revoked: ["access-1"],
    });
});

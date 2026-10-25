import { AccessContext, Caller, InstallationId, principal } from "@destack/access";
import { Identifier, present, schema } from "@destack/schema";
import { RequestId, type Subject } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import type { ObjectReference } from "@destack/sync";
import { DEVICE_CODE } from "../oauth/server.ts";
import { integration } from "../object/index.ts";
import { ConnectionFixture, testIssuer } from "../test/index.ts";

/** A space of another account, outside the primary's. */
const OUTSIDE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000f1");

/** Ask an integration's primary for a device authorization, token and revocation as a space. */
async function ask(primary: ConnectionFixture, registered: ObjectReference, space: string) {
    // call the primary as the space
    const client = primary.primary(space).primary(registered.scope);
    const target = {
        scope: registered.scope,
        id: schema.identifier("integration").parse(registered.id),
    };

    // start a device authorization, approve it, then request its token and revoke it
    const device = await client.integration.authorizeDevice({ ...target, request: { scopes: [] } });
    primary.server.approveDevice(device.userCode);
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
    // store a private and a public integration at their primary
    const primary = await ConnectionFixture.open();
    onTestFinished(() => primary.close());
    const owned = await primary.staticIntegration();
    const shared = await primary.staticIntegration({ isPublic: true });

    // call each as a space placed in another account
    const client = primary.primary(OUTSIDE).primary(owned.scope);
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
    const hidden = ["NOT_FOUND", "NOT_FOUND", `integration ${owned.id} not found`];
    expect({
        refused,
        served: await ask(primary, shared, OUTSIDE),
        revoked: primary.server.revoked,
    }).toEqual({
        refused: [hidden, hidden, hidden],
        served: { userCode: "USER-1", token: "access-1" },
        revoked: ["access-1"],
    });
});

test("let the installation issuing an integration's tokens read it, and refuse another installation", async () => {
    // register an integration issued by the serving space's declared issuer installation
    const fixture = await ConnectionFixture.open({ issuer: testIssuer.implement({ methods: {} }) });
    onTestFinished(() => fixture.close());
    const issuer = await fixture.createIssuer();
    const [created] = await fixture.objects.mutate(fixture.context().access(), {
        id: RequestId.create(fixture.environment),
        calls: [
            integration.call("create", {
                scope: fixture.scope.id,
                issuer,
                endpoints: { isIssuerReturned: false },
                clientId: "Iv1.app",
                authentication: "none",
                redirectUris: ["https://app.test/setup"],
            }),
        ],
    });

    // read the integration as the issuing installation and as another installation of its space
    const { scope } = issuer.issuer;
    const issuing = principal.installation.reference(
        scope,
        await InstallationId.declared({ id: scope }, issuer.name, fixture.environment.crypto),
    );
    const other = principal.installation.reference(
        scope,
        Identifier.create("installation", fixture.environment),
    );
    const id = present(created, "the integration").id;
    const read = (subject: Subject) =>
        fixture.objects.query(
            AccessContext.of(Caller.of(subject), fixture.environment.clock.now()),
            integration.call("get", { scope: fixture.scope.id, id }),
        );
    expect({ issuing: (await read(issuing)).id, other: await refusal(read(other)) }).toEqual({
        issuing: id,
        other: ["NOT_FOUND", "NOT_FOUND", `integration ${id} not found`],
    });
});

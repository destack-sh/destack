import { TokenResponse } from "../src/authentication/index.ts";
import { Snapshot } from "@destack/db";
import type { Crypto } from "@destack/environment";
import { Jwt } from "@destack/identity";
import { Relationship } from "@destack/access";
import { PackageId } from "@destack/package";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import { TOKEN_PATH } from "../src/authentication/path.ts";
import { account } from "../src/object/index.ts";
import { JWT_BEARER } from "../src/oauth/index.ts";
import accountPackage from "../src/package.ts";
import { ISSUER, OWNER_EMAIL, TestAccountStore } from "../src/test/index.ts";

/** The token endpoint, every federated token's audience. */
const ENDPOINT = `${ISSUER}${TOKEN_PATH}`;

/** The OIDC issuer of the workflows, as GitHub Actions is one. */
const WORKFLOW_ISSUER = "https://issuer.test";

/** The workflow the account trusts to publish. */
const WORKFLOW = "repo:acme/notes:environment:release";

/** The package the workflow asks a token for, as the registry is one. */
const AUDIENCE = PackageId.parse("package-019f7480-0000-7000-8000-00000000e003");

/** An RSA key of the issuer: its private half, and its public half as its key set publishes it. */
async function issuerKey(crypto: Crypto, kid: string) {
    const algorithm = {
        name: "RSASSA-PKCS1-v1_5",
        modulusLength: 2048,
        publicExponent: new Uint8Array([1, 0, 1]),
        hash: "SHA-256",
    };
    const pair = await crypto.generateKey(algorithm, true, ["sign", "verify"]);
    const jwk = await crypto.exportKey("jwk", pair.publicKey);

    return { privateKey: pair.privateKey, jwk: { ...jwk, kid, alg: "RS256", use: "sig" } };
}

test("grant a trusted workflow a token once by its issuer's OIDC token, refusing a replay, an untrusted workflow, another audience and a forged signature", async () => {
    // serve an OIDC issuer publishing its discovery document and one RS256 key
    await using fixture = await TestAccountStore.open();
    const { crypto, clock } = fixture.environment;
    const signing = await issuerKey(crypto, "issuer-key");
    const forger = await issuerKey(crypto, "issuer-key");
    const server = fixture.simulation.environment("issuer").server.http({
        hostname: "issuer.test",
        port: 443,
        fetch: (request) => {
            const { pathname } = new URL(request.url);

            return pathname === "/.well-known/openid-configuration"
                ? Response.json({
                      issuer: WORKFLOW_ISSUER,
                      jwks_uri: `${WORKFLOW_ISSUER}/.well-known/jwks`,
                      id_token_signing_alg_values_supported: ["RS256"],
                  })
                : Response.json({ keys: [signing.jwk] });
        },
    });

    // let the owner's account trust the workflow
    const owner = await fixture.signIn(OWNER_EMAIL);
    const owned = await fixture.createAccount(owner);
    const trusted = await owner.client(accountPackage).trustedPublisher.create({
        accountId: owned.id,
        requestId: RequestId.create(fixture.environment),
        issuer: WORKFLOW_ISSUER,
        subject: WORKFLOW,
    });

    // sign tokens as the issuer or a forger for a workflow and an audience, each with its own identifier
    let signed = 0;
    const sign = (key: CryptoKey, subject: string, audience: string) => {
        const now = Math.floor(clock.now() / 1000);
        signed += 1;
        const claims = {
            iss: WORKFLOW_ISSUER,
            sub: subject,
            aud: audience,
            iat: now,
            exp: now + 300,
            jti: `token-${signed}`,
        };

        return Jwt.sign(claims, { alg: "RS256", kid: "issuer-key" }, key, crypto);
    };

    // ask the token endpoint for a token by a signed token
    const ask = async (token: string) => {
        const response = await fixture.fetch(
            new Request(ENDPOINT, {
                method: "POST",
                body: new URLSearchParams({
                    grant_type: JWT_BEARER,
                    assertion: token,
                    resource: PackageId.resource(AUDIENCE),
                }),
            }),
        );
        const body: unknown = await response.json();

        return response.ok ? TokenResponse.parse(body).token_type : [response.status, body];
    };
    // grant the trusted workflow's token once, refuse the rest, and relate it as a publisher of its account
    const trustedToken = await sign(signing.privateKey, WORKFLOW, ENDPOINT);
    const answers = [
        await ask(trustedToken),
        await ask(trustedToken),
        await ask(await sign(signing.privateKey, "repo:acme/other:environment:release", ENDPOINT)),
        await ask(await sign(signing.privateKey, WORKFLOW, "https://elsewhere.test/token")),
        await ask(await sign(forger.privateKey, WORKFLOW, ENDPOINT)),
    ];
    const publishers = await Relationship.read(
        Snapshot.live(fixture.database),
        account.reference(owned.scope, owned.id),
        { relation: "publisher" },
    );
    expect([
        answers,
        publishers.map((related) => [related.subjectType, related.subjectId]),
    ]).toEqual([
        [
            "Bearer",
            refused("authentication required: assertion was already used"),
            refused("workflow is invalid: no account trusts this workflow"),
            refused("federated token is invalid: it does not verify"),
            refused("federated token is invalid: it does not verify"),
        ],
        [["trusted-publisher", trusted.id]],
    ]);
    await server.stop(true);
});

/** Describe the token endpoint's refusal of a grant, as the test reads it. */
function refused(description: string): unknown[] {
    return [400, { error: "invalid_grant", error_description: description }];
}

import { Jwt, KeyPair } from "@destack/identity";
import { PackageId } from "@destack/package";
import { RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import accountPackage from "../package.ts";
import { OWNER_EMAIL, TestAccountStore } from "../test/index.ts";
import { FederatedVerifier } from "./federation.ts";

/** A plain HTTP issuer, which only a universe answering over plain HTTP trusts. */
const PLAIN_ISSUER = "http://issuer.test";

/** The workflow the plain issuer signs for. */
const WORKFLOW = "repo:example/notes:environment:release";

test("refuse a plain HTTP issuer at a universe over HTTPS, before asking it for keys", async () => {
    // let an account trust a workflow of a plain HTTP issuer
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const owned = await fixture.createAccount(owner);
    await owner.client(accountPackage).client.trust({
        scope: owned.id,
        requestId: RequestId.create(fixture.environment),
        name: "Release",
        issuer: PLAIN_ISSUER,
        subject: WORKFLOW,
    });

    // verify a token of it at an HTTPS token endpoint, which no issuer server answers for
    const endpoint = "https://destack.test/oauth/token";
    const { crypto, clock } = fixture.environment;
    const now = Math.floor(clock.now() / 1000);
    const { pair } = await KeyPair.generate(crypto);
    const claims = {
        iss: PLAIN_ISSUER,
        sub: WORKFLOW,
        aud: endpoint,
        iat: now,
        exp: now + 300,
        jti: "token-1",
    };
    const token = await Jwt.sign(claims, { alg: "ES256" }, pair.privateKey, crypto);
    const verifier = new FederatedVerifier(fixture.database, endpoint, fixture.environment);
    const verified = verifier.verify(token, PackageId.parse(accountPackage.id), clock.now());

    // refuse it for its plain issuer
    expect(await refusal(verified)).toEqual([
        "BAD_REQUEST",
        "GRANT_INVALID",
        "issuer 'http://issuer.test' is invalid: a universe over HTTPS trusts HTTPS issuers alone",
    ]);
});

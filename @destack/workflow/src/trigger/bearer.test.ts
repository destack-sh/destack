import { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { Jwk } from "@destack/identity";
import { defineHttps } from "@destack/object";
import { TEST_START, testKeys, testSimulation } from "@destack/object/test";
import { BearerSignature } from "./bearer.ts";
import type { WebhookMessage } from "./webhook.ts";

/** The sender's API, whose key set the scheme reads. */
const sender = defineHttps("sender", { url: "https://sender.test" });

/** The issuer signing the pushes. */
const ISSUER = "https://issuer.test";

/** The ES256 key parameters of the sender's keys (RFC 7518 3.4). */
const SIGNING = { name: "ECDSA", namedCurve: "P-256" };

/** Verify pushes signed for one audience, identifying each delivery by its body's message. */
const signature = new BearerSignature({
    issuer: ISSUER,
    audience: "destack:push",
    keys: { resource: sender, path: "/keys" },
    delivery: (payload) => ({
        id: schema.object({ id: schema.string() }).loose().parse(payload).id,
        event: "push",
    }),
});

/** A push the issuer signs, sent at the test's start. */
function message(id: string): WebhookMessage {
    return { id, event: "push", body: JSON.stringify({ id }), sentAt: TEST_START };
}

test("verify a bearer token against the key set read through the installation's resource, once per freshness window, and refuse a token for another audience", async () => {
    // publish the sender's key set through a resource counting its reads
    const world = testSimulation().environment("bridge");
    const pair = await world.crypto.generateKey(SIGNING, true, ["sign", "verify"]);
    const jwk = { ...(await Jwk.export(pair.publicKey, world.crypto)), kid: "key-1", alg: "ES256" };
    let reads = 0;
    const resources = new ResourceContext().bind(sender, {
        url: "https://sender.test",
        fetch: async (path) => {
            reads += 1;

            return path === "/keys"
                ? Response.json({ keys: [jwk] })
                : new Response(null, { status: 404 });
        },
    });
    const context = {
        resources,
        keys: await testKeys(world),
        environment: world,
    };

    // sign two pushes as the issuer, and one for another audience
    const secret = JSON.stringify({
        ...(await Jwk.export(pair.privateKey, world.crypto)),
        kid: "key-1",
    });
    const post = async (id: string, signing = signature) =>
        new Request("https://bridge.test/.destack/webhook/push", {
            method: "POST",
            headers: await signing.sign(message(id), secret, world.crypto),
            body: JSON.stringify({ id }),
        });
    const other = new BearerSignature({ ...signature.options, audience: "destack:other" });

    const verify = signature.verifier(context);
    const delivered = [
        await verify(await post("m-1"), undefined, {}, TEST_START),
        await verify(await post("m-2"), undefined, {}, TEST_START),
    ].map((delivery) => [delivery.id, delivery.event]);
    const refused = await refusal(verify(await post("m-3", other), undefined, {}, TEST_START));

    expect({ delivered, reads, refused }).toEqual({
        delivered: [
            ["m-1", "push"],
            ["m-2", "push"],
        ],
        reads: 1,
        refused: ["UNAUTHORIZED", "AUDIENCE_REFUSED", "the token is for another audience"],
    });
});

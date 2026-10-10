import { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { exportJWK, generateKeyPair } from "jose";
import { defineHttps } from "../declare/index.ts";
import { CallKey } from "../request/index.ts";
import { testCallKey } from "../test/index.ts";
import { BearerSignature } from "./bearer.ts";

/** The sender's API, whose key set the scheme reads. */
const sender = defineHttps("sender", { url: "https://sender.test" });

/** The issuer signing the pushes. */
const ISSUER = "https://issuer.test";

/** A push's verification time. */
const NOW = Date.UTC(2026, 9, 7, 12);

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

test("verify a bearer token against the key set read through the installation's resource, once per freshness window, and refuse a token for another audience", async () => {
    // publish the sender's key set through a resource counting its reads
    const { privateKey, publicKey } = await generateKeyPair("ES256", { extractable: true });
    const jwk = { ...(await exportJWK(publicKey)), kid: "key-1", alg: "ES256" };
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
        deriver: await CallKey.deriver(await testCallKey()),
    };

    // sign two pushes as the issuer, and one for another audience
    const secret = JSON.stringify({ ...(await exportJWK(privateKey)), kid: "key-1" });
    const message = (id: string) => ({
        id,
        event: "push",
        body: JSON.stringify({ id }),
        sentAt: NOW,
    });
    const post = async (id: string, signing = signature) =>
        new Request("https://bridge.test/.destack/webhook/push", {
            method: "POST",
            headers: await signing.sign(message(id), secret),
            body: JSON.stringify({ id }),
        });
    const other = new BearerSignature({ ...signature.options, audience: "destack:other" });

    const delivered = [
        await signature.verify(await post("m-1"), undefined, {}, NOW, context),
        await signature.verify(await post("m-2"), undefined, {}, NOW, context),
    ].map((delivery) => [delivery.id, delivery.event]);
    const refused = await refusal(
        signature.verify(await post("m-3", other), undefined, {}, NOW, context),
    );

    expect({ delivered, reads, refused }).toEqual({
        delivered: [
            ["m-1", "push"],
            ["m-2", "push"],
        ],
        reads: 1,
        refused: ["UNAUTHORIZED", "webhook bearer token is not the issuer's for this endpoint"],
    });
});

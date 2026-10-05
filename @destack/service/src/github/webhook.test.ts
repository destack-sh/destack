import { expect, test } from "@destack/test";
import { GitHubSignature } from "./webhook.ts";

/** The GitHub example of "Testing the webhook payload validation". */
const GITHUB_EXAMPLE = {
    secret: "It's a Secret to Everybody",
    body: "Hello, World!",
    signature: "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17",
};

/** The delivery time. */
const NOW = 1614265390 * 1000;

/** Post a signed body. */
function post(headers: Headers, body: string): Request {
    return new Request("https://hooks.test/webhooks/github", { method: "POST", headers, body });
}

/** Read the body digest of a GitHub signature, which identifies its delivery. */
function digestOf(headers: Headers): string {
    // require the signature header
    const signature = headers.get("x-hub-signature-256");
    if (signature === null) {
        throw new TypeError("a GitHub delivery carries a signature");
    }

    return signature.slice("sha256=".length);
}

test("sign the GitHub reference example exactly", async () => {
    const github = await new GitHubSignature().sign(
        { id: "delivery-1", event: "push", body: GITHUB_EXAMPLE.body, sentAt: NOW },
        GITHUB_EXAMPLE.secret,
    );
    expect([...github.entries()]).toEqual([
        ["x-github-delivery", "delivery-1"],
        ["x-github-event", "push"],
        ["x-hub-signature-256", GITHUB_EXAMPLE.signature],
    ]);
});

test("verify a GitHub delivery and refuse a signature made with another secret", async () => {
    // accept the signed push
    const signature = new GitHubSignature();
    const body = '{"ref":"refs/heads/main","after":"aa218f56b14c9653891f9e74264a383fa43fefbd"}';
    const message = {
        id: "72d3162e-cc78-11e3-81ab-4c9367dc0958",
        event: "push",
        body,
        sentAt: NOW,
    };
    const headers = await signature.sign(message, GITHUB_EXAMPLE.secret);

    // know the delivery by its signed body, since GitHub signs no delivery header
    expect(await signature.verify(post(headers, body), GITHUB_EXAMPLE.secret, {}, NOW)).toEqual({
        id: digestOf(headers),
        event: "push",
        payload: { ref: "refs/heads/main", after: "aa218f56b14c9653891f9e74264a383fa43fefbd" },
        parameters: {},
        receivedAt: NOW,
    });

    // refuse another secret
    const forged = await signature.sign(message, "another secret");
    await expect(
        signature.verify(post(forged, body), GITHUB_EXAMPLE.secret, {}, NOW),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED", message: "webhook signature does not match" });
});

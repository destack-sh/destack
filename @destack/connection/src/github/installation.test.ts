import { schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { expect, onTestFinished, test } from "@destack/test";
import { jwtVerify } from "jose";
import { integration, connection } from "../object/index.ts";
import { ConnectionFixture } from "../test/index.ts";
import { GitHubIssuer } from "./installation.ts";

/** The DER length of a PKCS #8 RSA key's prefix before its PKCS #1 key: sequence, version, algorithm and octet string headers. */
const PKCS8_PREFIX_BYTES = 26;

/** Write a PEM of a label around DER bytes. */
function pem(label: string, der: Uint8Array): string {
    const lines = der.toBase64().match(/.{1,64}/gu) ?? [];

    return `-----BEGIN ${label}-----\n${lines.join("\n")}\n-----END ${label}-----\n`;
}

/** Generate a GitHub App's key pair, its private key as the PKCS #1 PEM GitHub issues. */
async function appKey(): Promise<{ readonly publicKey: CryptoKey; readonly privateKey: string }> {
    const pair = await crypto.subtle.generateKey(
        {
            name: "RSASSA-PKCS1-v1_5",
            modulusLength: 2048,
            publicExponent: new Uint8Array([1, 0, 1]),
            hash: "SHA-256",
        },
        true,
        ["sign", "verify"],
    );
    const pkcs8 = new Uint8Array(await crypto.subtle.exportKey("pkcs8", pair.privateKey));

    return {
        publicKey: pair.publicKey,
        privateKey: pem("RSA PRIVATE KEY", pkcs8.slice(PKCS8_PREFIX_BYTES)),
    };
}

/** Open a connection service beside GitHub's API, which verifies the app's JWT and records each call until the test ends. */
async function openGitHub(publicKey: CryptoKey, calls: string[]): Promise<ConnectionFixture> {
    // issue installation tokens through GitHub's API
    const opened = await ConnectionFixture.open({
        issuers: (fetch) => [
            new GitHubIssuer({
                fetch,
                api: "https://api.github.test",
                issuer: "https://github.test",
            }),
        ],
    });
    onTestFinished(() => opened.close());

    // answer GitHub's API, verifying the app's JWT
    opened.server.other = async (request) => {
        const token = String(request.headers.get("authorization")).slice("Bearer ".length);
        const { payload } = await jwtVerify(token, publicKey);
        calls.push(`${request.method} ${new URL(request.url).pathname} as ${payload.iss}`);

        return request.method === "DELETE"
            ? new Response(null, { status: 204 })
            : Response.json({ token: "ghs_installation", expires_at: "2099-01-01T00:00:00Z" });
    };

    return opened;
}

/** Register a GitHub App with its private key and create its installation connection. */
async function register(opened: ConnectionFixture, privateKey: string) {
    // register the app with its key
    const created = await opened.objects
        .mutate(opened.context().caller(), {
            id: RequestId.create(),
            calls: [
                integration.call("create", {
                    scope: opened.scope.id,
                    issuer: "https://github.test",
                    endpoints: {
                        installation: "https://github.test/apps/destack-test/installations/new",
                        isIssuerReturned: false,
                    },
                    clientId: "Iv1.app",
                    authentication: "private_key_jwt",
                    redirectUris: ["https://app.test/setup"],
                    vault: opened.vaultReference,
                    secret: privateKey,
                }),
            ],
        })
        .then(([written]) => written);

    // create the installation connection
    return opened.objects
        .mutate(opened.context().caller(), {
            id: RequestId.create(),
            calls: [
                connection.call("create", {
                    scope: opened.scope.id,
                    protocol: { kind: "installation" },
                    integration: integration.reference(opened.scope.id, created.id),
                    vault: opened.vaultReference,
                }),
            ],
        })
        .then(([written]) => written);
}

/** Install the app as a person does: start on the installation page and complete with GitHub's redirect, answering the page and its state. */
async function install(opened: ConnectionFixture, id: string) {
    // start on the installation page
    const { scope } = opened;
    const { url } = await opened.objects
        .mutate(opened.context().caller(), {
            id: RequestId.create(),
            calls: [
                connection.call("authorize", {
                    scope: scope.id,
                    id,
                    redirectUri: "https://app.test/setup",
                }),
            ],
        })
        .then(([written]) => written);
    const state = String(new URL(url).searchParams.get("state"));

    // complete with the installation GitHub redirects back with
    await opened.objects
        .mutate(opened.context().caller(), {
            id: RequestId.create(),
            calls: [
                connection.call("complete", {
                    scope: scope.id,
                    id,
                    parameters: { installation_id: "4242", setup_action: "install", state },
                }),
            ],
        })
        .then(([written]) => written);

    return { url, state };
}

test("install a GitHub App on an account, issue installation tokens with its PKCS #1 key, lend them, and uninstall it on revocation", async () => {
    // register and install the app beside GitHub's API
    const { publicKey, privateKey } = await appKey();
    const calls: string[] = [];
    const opened = await openGitHub(publicKey, calls);
    const reference = await register(opened, privateKey);
    const { url, state } = await install(opened, reference.id);

    // lend the installation token, then revoke the connection
    const loan = await opened.lender.lend(
        connection.reference(opened.scope.id, reference.id),
        new Request("https://api.github.test/repos/acme/app"),
        Date.now(),
    );
    await opened.objects
        .mutate(opened.context().caller(), {
            id: RequestId.create(),
            calls: [connection.call("revoke", { scope: opened.scope.id, id: reference.id })],
        })
        .then(([written]) => written);
    const [row] = await opened.connections();

    expect({
        page: new URL(url).origin + new URL(url).pathname,
        subject: row?.subject,
        lent: loan.request.headers.get("authorization"),
        calls,
        isState: schema.string().length(64).safeParse(state).success,
    }).toEqual({
        page: "https://github.test/apps/destack-test/installations/new",
        subject: "4242",
        lent: "token ghs_installation",
        calls: [
            "POST /app/installations/4242/access_tokens as Iv1.app",
            "DELETE /app/installations/4242 as Iv1.app",
        ],
        isState: true,
    });
});

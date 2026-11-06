import type { Environment } from "@destack/environment";
import { schema } from "@destack/schema";

/** The Cloudflare account running Destack, whose R2 bucket keeps every root's state. */
const ACCOUNT_ID = "27c0d00fb3a27a4ccbf46a3cceab9301";

/** How long Cloudflare may take to verify the token, in milliseconds. */
const VERIFY_MILLISECONDS = 15_000;

/** A successful verification of an active Cloudflare API token. */
const TokenVerification = schema
    .object({
        success: schema.literal(true),
        result: schema.object({ id: schema.string(), status: schema.literal("active") }).strip(),
    })
    .strip();

/** The Cloudflare credentials one OpenTofu deployment runs with. */
export interface CloudflareCredentials {
    /** Provider variables added to the inherited environment. */
    readonly environment: { readonly CLOUDFLARE_API_TOKEN: string };
    /** Backend settings signing the R2 state requests. */
    readonly backend: { readonly access_key: string; readonly secret_key: string };
}

/** The Cloudflare credentials one OpenTofu deployment runs with. */
export const CloudflareCredentials = {
    /** Resolve deployment credentials for Cloudflare and R2 state storage. */
    async resolve(environment: Environment<"process">): Promise<CloudflareCredentials> {
        // require the company Cloudflare token
        const token = environment.process.variables["CLOUDFLARE_COMPANY_API_TOKEN"];
        if (token === undefined || token === "") {
            throw new Error("CLOUDFLARE_COMPANY_API_TOKEN is required");
        }

        // derive the R2 credential from the verified account token
        const response = await environment.network.fetch(
            new Request(
                `https://api.cloudflare.com/client/v4/accounts/${ACCOUNT_ID}/tokens/verify`,
                {
                    headers: { Authorization: `Bearer ${token}` },
                    signal: environment.clock.timeout(VERIFY_MILLISECONDS),
                },
            ),
        );
        if (!response.ok) {
            throw new Error(`the Cloudflare token verification failed: ${response.status}`);
        }
        const verification = TokenVerification.safeParse(await response.json());
        if (!verification.success) {
            throw new Error("cloudflare deployment token is inactive or invalid", {
                cause: verification.error,
            });
        }

        // sign R2 state through backend settings: the token's id and the SHA-256 of its value, as R2 derives them
        const digest = await environment.crypto.digest("SHA-256", new TextEncoder().encode(token));

        return {
            environment: { CLOUDFLARE_API_TOKEN: token },
            backend: {
                access_key: verification.data.result.id,
                secret_key: new Uint8Array(digest).toHex(),
            },
        };
    },
};

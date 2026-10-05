import { createHash } from "node:crypto";
import { schema } from "@destack/schema";

/** A successful verification of an active Cloudflare API token. */
const TokenVerification = schema
    .object({
        success: schema.literal(true),
        result: schema.object({ id: schema.string(), status: schema.literal("active") }).strip(),
    })
    .strip();

/** Credentials one OpenTofu deployment runs with. */
export interface Credentials {
    /** Provider variables added to the inherited environment. */
    readonly environment: { readonly CLOUDFLARE_API_TOKEN: string };
    /** Backend settings signing the R2 state requests. */
    readonly backend: { readonly access_key: string; readonly secret_key: string };
}

/** Resolve deployment credentials for Cloudflare and R2 state storage. */
export async function credentials(): Promise<Credentials> {
    // require the company Cloudflare token
    const token = process.env["CLOUDFLARE_COMPANY_API_TOKEN"];
    if (token === undefined || token === "") {
        throw new Error("CLOUDFLARE_COMPANY_API_TOKEN is required");
    }

    // derive the R2 credential from the verified account token
    const response = await fetch(
        "https://api.cloudflare.com/client/v4/accounts/27c0d00fb3a27a4ccbf46a3cceab9301/tokens/verify",
        {
            headers: { Authorization: `Bearer ${token}` },
            signal: AbortSignal.timeout(15_000),
        },
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

    // sign R2 state through backend settings
    return {
        environment: { CLOUDFLARE_API_TOKEN: token },
        backend: {
            access_key: verification.data.result.id,
            secret_key: createHash("sha256").update(token).digest("hex"),
        },
    };
}

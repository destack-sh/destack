import { createHash } from "node:crypto";

/** Resolve existing deployment credentials for Cloudflare and R2 state storage. */
export async function credentials(): Promise<Record<string, string>> {
    // require the company Cloudflare token
    const token = process.env["CLOUDFLARE_COMPANY_API_TOKEN"];
    if (!token) {
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
        throw new Error(`Cloudflare token verification failed: ${response.status}.`);
    }
    const verification = (await response.json()) as {
        success: boolean;
        result?: { status: string; id: string };
    };
    if (
        !verification.success ||
        verification.result?.status !== "active" ||
        typeof verification.result.id !== "string"
    ) {
        throw new Error("cloudflare deployment token is inactive or invalid");
    }

    return {
        AWS_ACCESS_KEY_ID: verification.result.id,
        AWS_SECRET_ACCESS_KEY: createHash("sha256").update(token).digest("hex"),
        AWS_SESSION_TOKEN: "",
        CLOUDFLARE_API_TOKEN: token,
    };
}

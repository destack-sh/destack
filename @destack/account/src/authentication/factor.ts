import { twoFactor } from "better-auth/plugins";
import { createAuthMiddleware } from "better-auth/api";

/** Require enrolled second factors after every platform first-factor sign-in. */
export function secondFactor(challengeUri: string) {
    // configure the two-factor plugin
    const plugin = twoFactor({
        issuer: "Destack",
        allowPasswordless: true,
        backupCodeOptions: { storeBackupCodes: "encrypted" },
    });

    // reuse Better Auth's challenge, recovery, lockout, and trusted-device protocol
    const after = plugin.hooks.after.map((hook) => ({
        ...hook,
        matcher(context: Parameters<typeof hook.matcher>[0]) {
            return (
                hook.matcher(context) ||
                context.path === "/magic-link/verify" ||
                context.path === "/sign-in/email-otp" ||
                context.path?.startsWith("/callback/") === true
            );
        },
    }));

    // let Better Auth merge the challenge cookies before changing browser navigation
    after.push({
        matcher(context) {
            return (
                context.path === "/magic-link/verify" ||
                context.path?.startsWith("/callback/") === true
            );
        },
        handler: createAuthMiddleware(async (context) => {
            const result = context.context.returned;
            if (
                result &&
                typeof result === "object" &&
                "twoFactorRedirect" in result &&
                result.twoFactorRedirect === true
            ) {
                // carry the sign-in's own redirect on to the challenge
                const page = new URL(challengeUri);
                const location = context.context.responseHeaders?.get("location");
                if (location !== null && location !== undefined) {
                    page.searchParams.set("callbackURL", location);
                }
                throw context.redirect(page.href);
            }

            return undefined;
        }),
    });

    return { ...plugin, hooks: { ...plugin.hooks, after } };
}

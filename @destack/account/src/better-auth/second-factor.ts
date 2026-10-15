import { twoFactor } from "better-auth/plugins";
import { createAuthMiddleware } from "better-auth/api";

/** The Better Auth route a mailed sign-in link opens. */
const LINK_PATH = "/magic-link/verify";

/** The Better Auth route an emailed code signs in at. */
const CODE_PATH = "/sign-in/email-otp";

/** The prefix of the Better Auth routes a provider's sign-in returns to. */
const CALLBACK_PREFIX = "/callback/";

/** Require enrolled second factors after every platform first-factor sign-in. */
export function betterAuthSecondFactor(challengeUri: string) {
    // configure the two-factor plugin
    const plugin = twoFactor({
        issuer: "Destack",
        allowPasswordless: true,
        backupCodeOptions: { storeBackupCodes: "encrypted" },
    });

    // reuse Better Auth's challenge, recovery, lockout and trusted-device protocol
    const after = plugin.hooks.after.map((hook) => ({
        ...hook,
        matcher(context: Parameters<typeof hook.matcher>[0]) {
            return (
                hook.matcher(context) || context.path === CODE_PATH || isNavigation(context.path)
            );
        },
    }));

    // let Better Auth merge the challenge cookies before changing browser navigation
    after.push({
        matcher: (context) => isNavigation(context.path),
        handler: createAuthMiddleware(async (context) => {
            const result = context.context.returned;
            if (
                typeof result === "object" &&
                result !== null &&
                "twoFactorRedirect" in result &&
                result.twoFactorRedirect === true
            ) {
                // pass the sign-in's redirect on to the challenge
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

/** Report whether a route signs a browser in by navigation, through a link or a provider. */
function isNavigation(path: string | undefined): boolean {
    return path === LINK_PATH || path?.startsWith(CALLBACK_PREFIX) === true;
}

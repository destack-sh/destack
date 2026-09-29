import { createAuthClient } from "better-auth/client";
import { magicLinkClient, emailOTPClient, twoFactorClient } from "better-auth/client/plugins";
import { passkeyClient } from "@better-auth/passkey/client";
import {
    oauthDeviceAuthorizationClient,
    oauthProviderClient,
} from "@better-auth/oauth-provider/client";

/** Connect to platform sign-in. */
export function connectAuthentication(origin: string) {
    return createAuthClient({
        baseURL: origin,
        basePath: "/auth",
        plugins: [
            magicLinkClient(),
            oauthDeviceAuthorizationClient(),
            oauthProviderClient(),
            emailOTPClient(),
            twoFactorClient(),
            passkeyClient(),
        ],
    });
}

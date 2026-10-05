import type { AccountHandle } from "@destack/account/object";
import { RESIDENCIES } from "./residency.ts";

/** The handle of the platform's own account, which enrolls the regions' hosts. */
export const PLATFORM_HANDLE = "destack";

/** The suffix of the DNS labels Destack serves its development deployment under. */
const DEVELOPMENT_SUFFIX = "-development";

/** The handles the platform's own accounts hold, which the account service keeps for the platform organisation. */
export const PLATFORM_HANDLES: readonly AccountHandle[] = [
    // first-party publishers and platform nouns
    PLATFORM_HANDLE,
    "app",
    "template",
    "platform",
    "system",
    "root",
    "universe",
    "relay",
    "registry",
    "repository",
    "forge",
    "account",
    "accounts",

    // infrastructure and RFC 2142 mail roles
    "api",
    "www",
    "cdn",
    "download",
    "downloads",
    "status",
    "docs",
    "blog",
    "auth",
    "login",
    "logout",
    "signin",
    "signup",
    "oauth",
    "sso",
    "mail",
    "email",
    "noreply",
    "no-reply",
    "postmaster",
    "hostmaster",
    "webmaster",
    "abuse",
    "security",
    "support",
    "help",
    "info",
    "admin",
    "administrator",
    "billing",
    "finance",
    "legal",
    "privacy",
    "terms",

    // routes
    "new",
    "settings",
    "explore",
    "search",
    "home",
    "dashboard",
    "about",
    "pricing",
    "well-known",

    // DNS labels Destack serves under, with each residency's and region's code in both deployments
    "space",
    "development",
    `relay${DEVELOPMENT_SUFFIX}`,
    ...RESIDENCIES.flatMap((residency) => [
        residency.code,
        ...residency.regions.map((region) => region.code),
    ]).flatMap((code) => [code, `${code}${DEVELOPMENT_SUFFIX}`]),
];

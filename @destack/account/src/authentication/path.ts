/** The path of the issuer's key set below its URL (RFC 8414 jwks_uri). */
export const JWKS_PATH = "/.well-known/jwks.json";

/** The paths of the issuer's metadata below its URL (OIDC Discovery 4, RFC 8414 3). */
export const METADATA_PATHS = [
    "/.well-known/openid-configuration",
    "/.well-known/oauth-authorization-server",
] as const;

/** The path of the authorization endpoint below the issuer's URL (RFC 6749 3.1). */
export const AUTHORIZATION_PATH = "/oauth2/authorize";

/** The path of the token endpoint below the issuer's URL (RFC 6749 3.2). */
export const TOKEN_PATH = "/oauth2/token";

/** The path of the device authorization endpoint below the issuer's URL (RFC 8628 3.1). */
export const DEVICE_AUTHORIZATION_PATH = "/oauth2/device_authorization";

/** The path of the revocation endpoint below the issuer's URL (RFC 7009 2). */
export const REVOCATION_PATH = "/oauth2/revoke";

/** The path of the introspection endpoint below the issuer's URL (RFC 7662 2). */
export const INTROSPECTION_PATH = "/oauth2/introspect";

/** The path of the userinfo endpoint below the issuer's URL (OIDC Core 5.3). */
export const USER_INFO_PATH = "/oauth2/userinfo";

/** The path of the stored authorization requests a consent view reads and decides (Ory Hydra's). */
export const AUTHORIZATION_REQUEST_PATH = "/oauth2/requests/authorization";

/** The path of the signed-in person's consents, which their list of apps reads and revokes. */
export const CONSENT_PATH = "/oauth2/consents";

/** The path of the pending device sign-ins a verification view reads and decides. */
export const DEVICE_REQUEST_PATH = "/oauth2/requests/device";
/** The path of a sign-up's handle check below the issuer's URL. */
export const HANDLE_CHECK_PATH = "/sign-up/handle";
/** The path of the residencies a sign-up chooses from below the issuer's URL. */
export const RESIDENCY_CHOICE_PATH = "/sign-up/residency";
/** The path of where a signed-in person's Home answers, below the issuer's URL. */
export const HOME_LOCATION_PATH = "/home";
/** The path of the providers the issuer signs people in through, below the issuer's URL. */
export const SIGN_IN_PROVIDERS_PATH = "/sign-in/providers";

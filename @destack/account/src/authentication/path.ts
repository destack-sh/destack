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

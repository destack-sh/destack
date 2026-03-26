import {
    hasCookie,
    readOauthRequest,
    readRequestCookie,
    resolveRedirectMatch,
} from "../chunks/oauth-helpers.js";
const appRequest = readOauthRequest(
    "session=abc123; theme=dark",
    ["draft"],
    "https://destack.dev/docs/?q=1",
);
const appRedirect = resolveRedirectMatch("https://destack.dev/docs/?q=1", [
    "https://destack.dev/docs",
    "https://destack.dev/app",
]);
export const appOauthState = {
    redirectUri: appRequest.normalizedRedirectUri,
    sessionCookie: readRequestCookie("session=abc123; theme=dark", "session"),
    hasThemeCookie: hasCookie(appRequest.cookies, "theme"),
    requestBody: appRequest.requestBody,
    redirectAllowed: appRedirect.isRegisteredRedirect,
    redirectTarget: appRedirect.normalizedIncoming,
    cookieCount: Object.keys(appRequest.cookies).length,
};
//# sourceMappingURL=./app.js.map

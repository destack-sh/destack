import {
    hasCookie,
    readOauthRequest,
    readRequestCookie,
    resolveRedirectMatch,
} from "./oauth-helpers";

const dashboardRequest = readOauthRequest(
    "token=xyz; mode=preview",
    "preview",
    "https://destack.dev/app///",
);
const dashboardRedirect = resolveRedirectMatch("https://destack.dev/app///", [
    "https://destack.dev/settings",
    "https://destack.dev/app",
]);

/** The OAuth request state for the dashboard entry. */
export const dashboardOauthState = {
    redirectUri: dashboardRequest.normalizedRedirectUri,
    tokenCookie: readRequestCookie("token=xyz; mode=preview", "token"),
    hasModeCookie: hasCookie(dashboardRequest.cookies, "mode"),
    requestBody: dashboardRequest.requestBody,
    redirectAllowed: dashboardRedirect.isRegisteredRedirect,
    redirectTarget: dashboardRedirect.normalizedIncoming,
    cookieCount: Object.keys(dashboardRequest.cookies).length,
};

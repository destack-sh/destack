import { S3Error } from "./error.ts";

/** A CORS rule in R2's bucket CORS policy shape. */
export interface CorsRule {
    /** What the rule allows. */
    allowed: {
        /** The allowed origins, each with at most one `*` wildcard. */
        origins: string[];
        /** The allowed methods: GET, PUT, POST, DELETE or HEAD. */
        methods: string[];
        /** The allowed request headers, each with at most one `*` wildcard. */
        headers?: string[];
    };
    /** The response headers browsers may read. */
    exposeHeaders?: string[];
    /** How long browsers may cache a preflight response, in seconds. */
    maxAgeSeconds?: number;
}

/** A CORS rule in R2's bucket CORS policy shape. */
export const CorsRule = { preflight, apply };

/** Answer a preflight request with the first rule allowing its origin, method and headers. */
function preflight(rules: CorsRule[], request: Request): Response {
    // require the origin and method a preflight names
    const origin = request.headers.get("origin");
    const method = request.headers.get("access-control-request-method");
    if (origin === null || method === null) {
        throw new S3Error(
            "BadRequest",
            "a preflight request requires the origin and access-control-request-method headers",
        );
    }
    const requested = (request.headers.get("access-control-request-headers") ?? "")
        .split(",")
        .map((header) => header.trim().toLowerCase())
        .filter((header) => header !== "");

    // select the first rule allowing everything the preflight asks for
    const rule = rules.find(
        (rule) =>
            allowsOrigin(rule, origin) &&
            rule.allowed.methods.includes(method) &&
            requested.every((header) =>
                (rule.allowed.headers ?? []).some((pattern) =>
                    matches(pattern.toLowerCase(), header),
                ),
            ),
    );
    if (rule === undefined) {
        throw new S3Error("AccessForbidden", "this CORS request is not allowed");
    }

    // grant the origin, the rule's methods and the requested headers
    const headers = new Headers({
        "access-control-allow-origin": allowedOrigin(rule, origin),
        "access-control-allow-methods": rule.allowed.methods.join(", "),
        vary: "Origin, Access-Control-Request-Headers, Access-Control-Request-Method",
    });
    if (requested.length > 0) {
        headers.set("access-control-allow-headers", requested.join(", "));
    }
    if (rule.exposeHeaders !== undefined && rule.exposeHeaders.length > 0) {
        headers.set("access-control-expose-headers", rule.exposeHeaders.join(", "));
    }
    if (rule.maxAgeSeconds !== undefined) {
        headers.set("access-control-max-age", String(rule.maxAgeSeconds));
    }

    return new Response(null, { status: 200, headers });
}

/** Add the headers of the first rule allowing a request's origin and method to its response. */
function apply(rules: CorsRule[], request: Request, headers: Headers): void {
    // leave requests without an origin, or without an allowing rule, as they are
    const origin = request.headers.get("origin");
    if (origin === null) {
        return;
    }
    headers.append("vary", "Origin");
    const rule = rules.find(
        (rule) => allowsOrigin(rule, origin) && rule.allowed.methods.includes(request.method),
    );
    if (rule === undefined) {
        return;
    }

    // grant the origin and expose the rule's headers
    headers.set("access-control-allow-origin", allowedOrigin(rule, origin));
    headers.set("access-control-allow-methods", rule.allowed.methods.join(", "));
    if (rule.exposeHeaders !== undefined && rule.exposeHeaders.length > 0) {
        headers.set("access-control-expose-headers", rule.exposeHeaders.join(", "));
    }
}

/** Whether a rule allows an origin. */
function allowsOrigin(rule: CorsRule, origin: string): boolean {
    return rule.allowed.origins.some((pattern) => matches(pattern, origin));
}

/** The allowed origin a response names: any origin for a lone wildcard, else the request's. */
function allowedOrigin(rule: CorsRule, origin: string): string {
    return rule.allowed.origins.includes("*") ? "*" : origin;
}

/** Match a value against a pattern with at most one `*` wildcard. */
function matches(pattern: string, value: string): boolean {
    // compare literally without a wildcard
    const wildcard = pattern.indexOf("*");
    if (wildcard === -1) {
        return pattern === value;
    }
    const prefix = pattern.slice(0, wildcard);
    const suffix = pattern.slice(wildcard + 1);

    return (
        value.length >= prefix.length + suffix.length &&
        value.startsWith(prefix) &&
        value.endsWith(suffix)
    );
}

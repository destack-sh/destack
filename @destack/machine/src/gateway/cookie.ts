import { HostAddress } from "@destack/environment";

/** The prefix binding a cookie to its HTTPS origin and root path (RFC 6265bis `__Host-`). */
const HOST_PREFIX = "__Host-";

/** A cookie of an installation's origin, `__Host-` and Secure except on this computer. */
export class OriginCookie {
    /** The cookie's name at its origin. */
    readonly name: string;
    /** Whether the origin is secure: any but this computer's loopback. */
    readonly isSecure: boolean;

    /** Bind a cookie's name to an origin. */
    constructor(name: string, url: URL) {
        this.isSecure = !HostAddress.isLoopbackName(url.hostname);
        this.name = this.isSecure ? `${HOST_PREFIX}${name}` : name;
    }

    /** Read the cookie's value from a request, absent without it. */
    read(request: Request): string | undefined {
        const found = (request.headers.get("cookie") ?? "")
            .split(";")
            .map((entry) => entry.trim())
            .find((entry) => entry.startsWith(`${this.name}=`));

        return found?.slice(this.name.length + 1);
    }

    /** Write the cookie's `Set-Cookie` value for some seconds, else the browser's session. */
    write(value: string, seconds?: number): string {
        const lifetime = seconds === undefined ? "" : `; Max-Age=${seconds}`;
        const secure = this.isSecure ? "; Secure" : "";

        return `${this.name}=${value}; Path=/; HttpOnly; SameSite=Lax${secure}${lifetime}`;
    }

    /** Write the `Set-Cookie` value clearing the cookie. */
    clear(): string {
        return this.write("", 0);
    }
}

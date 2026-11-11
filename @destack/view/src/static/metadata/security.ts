import { Duration } from "@destack/schema";
import { SecurityOptions } from "./site.ts";
import { ViewError } from "../../error/error.ts";

/** The longest a security.txt stays valid, a year of 365 days as RFC 9116 recommends less than. */
const MAX_LIFETIME = Duration.milliseconds({ days: 365 });

/** Write a site's security.txt expiring its lifetime after now, refusing a year or more. */
export function writeSecurity(origin: string, options: SecurityOptions, now: Date): string {
    // require a lifetime under a year
    const lifetime = Duration.milliseconds(options.lifetime);
    if (lifetime <= 0 || lifetime >= MAX_LIFETIME) {
        throw ViewError.of(
            "SECURITY_LIFETIME_INVALID",
            `security.txt lifetime ${JSON.stringify(options.lifetime)} is invalid: it must be less than a year`,
        );
    }
    const expires = new Date(now.getTime() + lifetime);

    // write each field on its own line, canonical at the well-known address
    return `${[
        ...options.contact.map((contact) => `Contact: ${contact}`),
        `Expires: ${expires.toISOString()}`,
        ...(options.preferredLanguages === undefined
            ? []
            : [`Preferred-Languages: ${options.preferredLanguages.join(", ")}`]),
        ...(options.policy === undefined ? [] : [`Policy: ${options.policy}`]),
        `Canonical: ${new URL("/.well-known/security.txt", origin).href}`,
    ].join("\n")}\n`;
}

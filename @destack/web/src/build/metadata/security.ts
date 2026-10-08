import { schema } from "@destack/schema";

/** A site's security contact (RFC 9116), published at `/.well-known/security.txt`. */
export const SecurityOptions = schema.object({
    /** Where to report vulnerabilities, as `mailto:` or `https:` addresses, preferred first. */
    contact: schema.array(schema.string().min(1)).min(1).readonly(),
    /** When the file stops being valid, as an ISO date and time less than a year ahead. */
    expires: schema.string().min(1),
    /** The languages reports may use, such as `en`. */
    preferredLanguages: schema.array(schema.string().min(1)).readonly().exactOptional(),
    /** The address of the disclosure policy. */
    policy: schema.string().exactOptional(),
});
/** A site's security contact. */
export type SecurityOptions = schema.Infer<typeof SecurityOptions>;

/** Write a site's security.txt, refusing an expiry that has passed. */
export function writeSecurity(origin: string, options: SecurityOptions, now: Date): string {
    // require a valid expiry in the future
    const expires = new Date(options.expires);
    if (Number.isNaN(expires.getTime()) || expires <= now) {
        throw new Error(`security.txt expires in the past or never: ${options.expires}`);
    }

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

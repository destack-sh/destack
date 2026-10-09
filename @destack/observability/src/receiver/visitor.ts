import { type DatabaseConnection, eq, ne } from "@destack/db";
import { present } from "@destack/schema";
import { visitorSalt } from "../stack/db.ts";

/** The salt bytes of one day. */
const SALT_BYTES = 32;

/** The hexadecimal digits of a visitor's hash. */
const VISITOR_DIGITS = 32;

/** The browser a visit came from, as the server forwarding its page's export saw it. */
export interface VisitorOrigin {
    /** The installation whose view was opened. */
    readonly installation: string;
    /** The browser's address. */
    readonly address: string;
    /** The browser's user agent. */
    readonly userAgent: string;
}

/** The visitors of a space's views, each a hash of the day's salt and the browser, the salt deleted once its day ends. */
export const VisitorSalt = {
    /** Hash a browser under the salt of the day a time falls in, creating the day's salt once and deleting every earlier day's then. */
    async hash(database: DatabaseConnection, now: number, origin: VisitorOrigin): Promise<string> {
        // read the day's salt, creating it once
        const day = new Date(now).toISOString().slice(0, "2026-10-07".length);
        const [held] = await database.select().from(visitorSalt).where(eq(visitorSalt.day, day));
        const row = held ?? (await createSalt(database, day));

        // hash the salt with the browser
        const text = [row.salt, origin.installation, origin.address, origin.userAgent].join("\n");
        const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));

        return new Uint8Array(digest).toHex().slice(0, VISITOR_DIGITS);
    },
};

/** Create a day's salt once, deleting every other day's so no earlier hash can be made again, and read the day's salt kept. */
async function createSalt(database: DatabaseConnection, day: string) {
    // insert the day's salt unless another receiver did first
    await database
        .insert(visitorSalt)
        .values({ day, salt: crypto.getRandomValues(new Uint8Array(SALT_BYTES)).toHex() })
        .onConflictDoNothing();

    // delete every other day's salt
    await database.delete(visitorSalt).where(ne(visitorSalt.day, day));

    // read the salt kept
    const [row] = await database.select().from(visitorSalt).where(eq(visitorSalt.day, day));

    return present(row, `the visitor salt of ${day}`);
}

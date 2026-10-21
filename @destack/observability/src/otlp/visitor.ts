import { Snapshot } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { CallError } from "@destack/object";
import { RequestId } from "@destack/sync";
import { type Identifier, present } from "@destack/schema";
import { visitorSalt, type VisitorSalt } from "../object/visitor-salt.ts";

/** The salt bytes of one day. */
const SALT_BYTES = 32;

/** The hexadecimal digits of a visitor's hash. */
const VISITOR_DIGITS = 32;

/** The length of a UTC day as ISO 8601 writes it, such as 2026-10-07. */
const DAY_LENGTH = 10;

/** The browser a visit came from, as the machine forwarding its page's export saw it. */
export interface VisitorOrigin {
    /** The installation whose view was opened. */
    readonly installation: string;
    /** The browser's address. */
    readonly address: string;
    /** The browser's user agent. */
    readonly userAgent: string;
}

/** A space's anonymous visitors, hashed under one salt per day. */
export class Visitor {
    /** The store of the space's salts. */
    readonly #store: Pick<ObjectStore, "database" | "serving" | "mutate">;
    /** The space. */
    readonly #space: Identifier<"space">;

    /** Hash a space's visitors through its store. */
    constructor(
        store: Pick<ObjectStore, "database" | "serving" | "mutate">,
        space: Identifier<"space">,
    ) {
        this.#store = store;
        this.#space = space;
    }

    /** Hash a browser under the salt of the day a time falls in. */
    async hash(origin: VisitorOrigin, now: number): Promise<string> {
        // hash the day's salt with the browser
        const salt = await this.#salt(new Date(now).toISOString().slice(0, DAY_LENGTH), now);
        const text = [salt.salt, origin.installation, origin.address, origin.userAgent].join("\n");
        const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));

        return new Uint8Array(digest).toHex().slice(0, VISITOR_DIGITS);
    }

    /** Read a day's salt, creating it on the day's first visit. */
    async #salt(day: string, now: number): Promise<VisitorSalt> {
        // read the day's salt
        const known = await this.#read(day);
        if (known !== undefined) {
            return known;
        }

        // create it, reading the one a concurrent intake created first
        const salt = crypto.getRandomValues(new Uint8Array(SALT_BYTES)).toHex();
        const create = visitorSalt.call("create", { ...visitorSalt.in(this.#space), day, salt });
        try {
            const [created] = await this.#store.mutate(this.#store.serving(visitorSalt), {
                id: RequestId.create(now),
                calls: [create],
            });

            return created;
        } catch (error) {
            if (CallError.of(error)?.code !== "CONFLICT") {
                throw error;
            }

            return present(await this.#read(day), `the visitor salt of ${day}`);
        }
    }

    /** Read the space's salt of a day. */
    #read(day: string): Promise<VisitorSalt | undefined> {
        return Snapshot.live(this.#store.database)
            .read(visitorSalt.table)
            .findFirst({ where: { scope: this.#space, day } });
    }
}

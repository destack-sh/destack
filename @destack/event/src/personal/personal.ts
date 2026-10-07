import { and, defineTable, desc, eq, integer, text, type DatabaseConnection } from "@destack/db";
import { Ciphertext, type Keyring } from "@destack/identity";
import { JsonValue, present, schema } from "@destack/schema";
import { v7 } from "uuid";
import type { EventKind } from "../kind/kind.ts";

/** The keys sealing people's personal values, gone once a person is forgotten. */
export interface PersonalKeyring {
    /** Read a person's current key, creating one when missing. */
    currentKey(
        subject: string,
        transaction?: DatabaseConnection,
    ): Promise<{ readonly id: string; readonly key: CryptoKey }>;
    /** Read one of a person's keys, absent once the person is forgotten. */
    key(subject: string, id: string): Promise<CryptoKey | undefined>;
    /** Forget a person's keys, erasing their personal fields in every event sealed under them. */
    forget(subject: string): Promise<void>;
}

/** The keys of the people a database's events name, each encrypted under the host's keyring. */
export const personalKey = defineTable("personal_key", {
    /** The person. */
    subject: text("subject").primaryKey(),
    /** The key's identity. */
    id: text("id").primaryKey(),
    /** The AES-256 key's bytes, encrypted under the host's keyring. */
    sealed: text("sealed").notNull(),
    /** When the key was created, in Unix milliseconds. */
    createdAt: integer("created_at").notNull(),
});

/** People's keys kept in a database, encrypted under the host's keyring, for the stores sharing that database. */
export class DatabasePersonalKeyring implements PersonalKeyring {
    /** The database keeping the keys. */
    readonly #database: DatabaseConnection;
    /** The host's keyring encrypting each person's key. */
    readonly #keyring: Keyring;

    /** Keep keys in a database under a host's keyring. */
    constructor(database: DatabaseConnection, keyring: Keyring) {
        this.#database = database;
        this.#keyring = keyring;
    }

    /** Read a person's newest key, creating one when missing. */
    async currentKey(
        subject: string,
        transaction?: DatabaseConnection,
    ): Promise<{ readonly id: string; readonly key: CryptoKey }> {
        // reuse the newest key
        const connection =
            transaction?.state === this.#database.state ? transaction : this.#database;
        const [kept] = await connection
            .select()
            .from(personalKey)
            .where(eq(personalKey.subject, subject))
            .orderBy(desc(personalKey.createdAt))
            .limit(1);
        if (kept !== undefined) {
            return { id: kept.id, key: await this.#open(subject, kept.id, kept.sealed) };
        }

        // create a key and keep it encrypted under the keyring
        const id = v7();
        const bytes = crypto.getRandomValues(new Uint8Array(32));
        const sealed = await this.#keyring.encrypt(bytes, contextOf(subject, id));
        await connection.insert(personalKey).values({ subject, id, sealed, createdAt: Date.now() });

        return { id, key: await importKey(bytes) };
    }

    /** Read one of a person's keys, absent once the person is forgotten. */
    async key(subject: string, id: string): Promise<CryptoKey | undefined> {
        const [kept] = await this.#database
            .select({ sealed: personalKey.sealed })
            .from(personalKey)
            .where(and(eq(personalKey.subject, subject), eq(personalKey.id, id)));

        return kept === undefined ? undefined : this.#open(subject, id, kept.sealed);
    }

    /** Delete a person's keys. */
    async forget(subject: string): Promise<void> {
        await this.#database.delete(personalKey).where(eq(personalKey.subject, subject));
    }

    /** Decrypt a kept key under the keyring. */
    async #open(subject: string, id: string, sealed: string): Promise<CryptoKey> {
        return importKey(
            await this.#keyring.decrypt(Ciphertext.parse(sealed), contextOf(subject, id)),
        );
    }
}

/** The sealing of an event's personal values and the dropping of its secrets. */
export const PersonalSeal = {
    /** Seal the data's personal values and drop its secrets. */
    async seal(
        kind: EventKind,
        keys: Readonly<Record<string, unknown>>,
        data: JsonValue,
        keyring: PersonalKeyring | undefined,
        transaction?: DatabaseConnection,
    ): Promise<JsonValue> {
        // mark each personal value and drop each secret
        const subject = kind.subject === undefined ? undefined : keys[kind.subject];
        const held = new Map<object, JsonValue>();
        const marked = kind.mapSensitive(data, (value, sensitivity) => {
            if (sensitivity === "secret" || typeof subject !== "string") {
                return undefined;
            }
            const marker = {};
            held.set(marker, value);

            return marker;
        });
        if (held.size === 0 || typeof subject !== "string") {
            return marked;
        }

        // encrypt each personal value's JSON under the person's current key, naming the key
        const { id, key } = await required(keyring, kind).currentKey(subject, transaction);
        const sealed = new Map<object, JsonValue>();
        for (const [marker, value] of held) {
            const plain = new TextEncoder().encode(JSON.stringify(value));
            const ciphertext = await Ciphertext.encrypt(
                plain,
                key,
                { alg: "A256GCMKW", kid: id },
                valueContext(kind.key, subject),
            );
            sealed.set(marker, { $sealed: ciphertext });
        }

        return replaced(marked, sealed);
    },

    /** Open the data's sealed personal values, leaving out forgotten people's. */
    async open(
        kind: EventKind,
        keys: Readonly<Record<string, unknown>>,
        data: JsonValue,
        keyring: PersonalKeyring | undefined,
    ): Promise<JsonValue> {
        // mark each sealed personal value
        const subject = kind.subject === undefined ? undefined : keys[kind.subject];
        const held = new Map<object, Ciphertext>();
        const marked = kind.mapSensitive(data, (value, sensitivity) => {
            // keep a value that is not a sealed personal one as it is
            const sealed =
                sensitivity === "personal" && JsonValue.isObject(value)
                    ? value["$sealed"]
                    : undefined;
            const parsed = Ciphertext.safeParse(sealed);
            if (!parsed.success) {
                return value;
            }
            const marker = {};
            held.set(marker, parsed.data);

            return marker;
        });
        if (held.size === 0 || typeof subject !== "string") {
            return marked;
        }

        // decrypt each under the key it names, leaving it out once the key is forgotten
        const opened = new Map<object, JsonValue | undefined>();
        for (const [marker, ciphertext] of held) {
            const id = present(Ciphertext.keyId(ciphertext), "a sealed value's key");
            const key = await required(keyring, kind).key(subject, id);
            const plain =
                key === undefined
                    ? undefined
                    : await Ciphertext.decrypt(ciphertext, key, valueContext(kind.key, subject));
            opened.set(
                marker,
                plain === undefined
                    ? undefined
                    : schema.json().parse(JSON.parse(new TextDecoder().decode(plain))),
            );
        }

        return replaced(marked, opened);
    },
};

/** Replace the marker objects in a JSON value by their values, leaving out those without one. */
function replaced(value: JsonValue, values: ReadonlyMap<object, JsonValue | undefined>): JsonValue {
    // replace a marker, then look into arrays and objects
    if (typeof value === "object" && value !== null && values.has(value)) {
        return values.get(value) ?? null;
    } else if (JsonValue.isArray(value)) {
        return value.map((entry) => replaced(entry, values));
    } else if (JsonValue.isObject(value)) {
        return Object.fromEntries(
            Object.entries(value).flatMap(([name, entry]) =>
                isGone(entry, values) ? [] : [[name, replaced(entry, values)]],
            ),
        );
    } else {
        return value;
    }
}

/** Report whether a value is a marker without a value, which its object leaves out. */
function isGone(value: JsonValue, values: ReadonlyMap<object, JsonValue | undefined>): boolean {
    return (
        typeof value === "object" &&
        value !== null &&
        values.has(value) &&
        values.get(value) === undefined
    );
}

/** Import a person's key from its bytes. */
function importKey(bytes: Uint8Array<ArrayBuffer>): Promise<CryptoKey> {
    return crypto.subtle.importKey("raw", bytes, { name: "AES-GCM", length: 256 }, false, [
        "encrypt",
        "decrypt",
    ]);
}

/** Bind a person's key to the person and its identity. */
function contextOf(subject: string, id: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(`personal-key ${subject} ${id}`);
}

/** Bind a sealed value to its kind and person. */
function valueContext(kind: string, subject: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(`personal-value ${kind} ${subject}`);
}

/** Require the personal keys a kind with personal fields needs. */
function required(personal: PersonalKeyring | undefined, kind: EventKind): PersonalKeyring {
    if (personal === undefined) {
        throw new TypeError(
            `event kind ${kind.key} seals personal fields, which needs personal keys`,
        );
    }

    return personal;
}

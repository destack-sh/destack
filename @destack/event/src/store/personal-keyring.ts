import type { Crypto, Environment } from "@destack/environment";
import { and, defineTable, desc, eq, integer, text, type DatabaseConnection } from "@destack/db";
import { Ciphertext, type Keyring, type Label } from "@destack/identity";
import { JsonValue, present, schema, Uuid } from "@destack/schema";
import type { EventKind } from "../kind/kind.ts";

/** The keys sealing people's personal values, gone once a person is forgotten. */
export interface PersonalKeyring {
    /** Read a person's current key, creating one when missing. */
    readonly currentKey: (
        subject: string,
        transaction?: DatabaseConnection,
    ) => Promise<{ readonly id: string; readonly key: CryptoKey }>;
    /** Read one of a person's keys, absent once the person is forgotten. */
    readonly key: (subject: string, id: string) => Promise<CryptoKey | undefined>;
    /** Forget a person's keys, erasing their personal fields in every event sealed under them. */
    readonly forget: (subject: string) => Promise<void>;
    /** The WebCrypto and randomness sealing the people's values. */
    readonly environment: Pick<Environment, "crypto" | "random">;
}

/** The keys of the people in a database's events, each encrypted under the host's keyring. */
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

/** People's keys in a database, encrypted under the host's keyring, for stores sharing it. */
export class DatabasePersonalKeyring implements PersonalKeyring {
    /** The database that stores the keys. */
    readonly #database: DatabaseConnection;
    /** The host's keyring encrypting each person's key. */
    readonly #keyring: Keyring;
    /** The WebCrypto and randomness of the people's keys. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "random">;

    /** Store keys in a database under a host's keyring. */
    constructor(
        database: DatabaseConnection,
        keyring: Keyring,
        environment: Pick<Environment, "clock" | "crypto" | "random">,
    ) {
        this.#database = database;
        this.#keyring = keyring;
        this.environment = environment;
    }

    /** Read a person's newest key, creating one when missing. */
    async currentKey(
        subject: string,
        transaction?: DatabaseConnection,
    ): Promise<{ readonly id: string; readonly key: CryptoKey }> {
        // reuse the newest key
        const connection =
            transaction?.state === this.#database.state ? transaction : this.#database;
        const [stored] = await connection
            .select()
            .from(personalKey)
            .where(eq(personalKey.subject, subject))
            .orderBy(desc(personalKey.createdAt))
            .limit(1);
        if (stored !== undefined) {
            return { id: stored.id, key: await this.#open(subject, stored.id, stored.sealed) };
        }

        // create a key and store it encrypted under the keyring
        const id = Uuid.v7(this.environment);
        const bytes = this.environment.random.bytes(32);
        const sealed = await this.#keyring.encrypt(bytes, encryptionContext(subject, id));
        await connection
            .insert(personalKey)
            .values({ subject, id, sealed, createdAt: this.environment.clock.now() });

        return { id, key: await importKey(bytes, this.environment.crypto) };
    }

    /** Read one of a person's keys, absent once the person is forgotten. */
    async key(subject: string, id: string): Promise<CryptoKey | undefined> {
        const [stored] = await this.#database
            .select({ sealed: personalKey.sealed })
            .from(personalKey)
            .where(and(eq(personalKey.subject, subject), eq(personalKey.id, id)));

        return stored === undefined ? undefined : this.#open(subject, id, stored.sealed);
    }

    /** Delete a person's keys. */
    async forget(subject: string): Promise<void> {
        await this.#database.delete(personalKey).where(eq(personalKey.subject, subject));
    }

    /** Decrypt a stored key under the keyring. */
    async #open(subject: string, id: string, sealed: string): Promise<CryptoKey> {
        return importKey(
            await this.#keyring.decrypt(Ciphertext.parse(sealed), encryptionContext(subject, id)),
            this.environment.crypto,
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
        const subject = kind.subjectOf(keys);
        const held = new Map<object, JsonValue>();
        const marked = kind.mapSensitive(data, (value, sensitivity) => {
            if (sensitivity === "secret" || subject === undefined) {
                return undefined;
            }
            const marker = {};
            held.set(marker, value);

            return marker;
        });
        if (held.size === 0 || subject === undefined) {
            return marked;
        }

        // encrypt each personal value's JSON under the person's current key and its identifier
        const personal = required(keyring, kind);
        const { id, key } = await personal.currentKey(subject, transaction);
        const sealed = new Map<object, JsonValue>();
        for (const [marker, value] of held) {
            const plain = new TextEncoder().encode(JSON.stringify(value));
            const ciphertext = await Ciphertext.encrypt(
                plain,
                key,
                { alg: "A256GCMKW", kid: id },
                valueContext(kind.key, subject),
                personal.environment,
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
        const subject = kind.subjectOf(keys);
        const held = new Map<object, Ciphertext>();
        const marked = kind.mapSensitive(data, (value, sensitivity) => {
            // return a value that is not a sealed personal one as it is
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
        if (held.size === 0 || subject === undefined) {
            return marked;
        }

        // decrypt each under its key, leaving it out once the key is forgotten
        const opened = new Map<object, JsonValue | undefined>();
        for (const [marker, ciphertext] of held) {
            const id = present(Ciphertext.keyId(ciphertext), "a sealed value's key");
            const personal = required(keyring, kind);
            const key = await personal.key(subject, id);
            const plain =
                key === undefined
                    ? undefined
                    : await Ciphertext.decrypt(
                          ciphertext,
                          key,
                          valueContext(kind.key, subject),
                          personal.environment.crypto,
                      );
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
            JsonValue.entries(value).flatMap(([name, entry]) =>
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
function importKey(bytes: Uint8Array<ArrayBuffer>, crypto: Crypto): Promise<CryptoKey> {
    return crypto.importKey("raw", bytes, { name: "AES-GCM", length: 256 }, false, [
        "encrypt",
        "decrypt",
    ]);
}

/** Label a person's key by the person and its identifier. */
function encryptionContext(subject: string, id: string): Label {
    return { label: "@destack/event/personal-key", context: `${subject} ${id}` };
}

/** Label a sealed value by its kind and person. */
function valueContext(kind: string, subject: string): Label {
    return { label: "@destack/event/personal-value", context: `${kind} ${subject}` };
}

/** Require the personal keys a kind with personal fields needs. */
function required(personal: PersonalKeyring | undefined, kind: EventKind): PersonalKeyring {
    if (personal === undefined) {
        throw new TypeError(
            `event kind ${kind.key} encrypts personal fields, so it needs personal keys`,
        );
    }

    return personal;
}

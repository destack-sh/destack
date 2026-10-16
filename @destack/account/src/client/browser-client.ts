import { principal } from "@destack/access";
import { Proof, PROOF_HEADER, PublicKey } from "@destack/identity";
import { Identifier, schema } from "@destack/schema";
import { Scope, type Subject } from "@destack/sync";
import type { Fetch } from "@destack/runtime";
import type { ClientRegistration } from "../object/client.ts";
import { KeyRegistration } from "../object/key.ts";
import { Platform } from "../object/platform.ts";

/** The IndexedDB database and object store of the browser's client. */
const DATABASE = "destack-client";

/** The key of the client in the store. */
const RECORD = "client";

/** The status a host answers a client with once the person removed it, RFC 9110 15.5.11. */
const GONE = 410;

/** The status a host answers a new registration with, RFC 9110 15.3.2. */
const CREATED = 201;

/** The status a host answers a registered key with, RFC 9110 15.3.1. */
const OK = 200;

/** The browsers in a user agent string without client hints, the more specific first. */
const USER_AGENT_BRANDS: readonly (readonly [RegExp, string])[] = [
    [/Firefox\//u, "Firefox"],
    [/Edg\//u, "Edge"],
    [/Chrome\//u, "Chrome"],
    [/Safari\//u, "Safari"],
];

/** The platforms in a user agent string without client hints, the more specific first. */
const USER_AGENT_PLATFORMS: readonly (readonly [RegExp, Platform])[] = [
    [/iPhone|iPad/u, "iOS"],
    [/Android/u, "Android"],
    [/Mac OS X/u, "macOS"],
    [/Windows/u, "Windows"],
    [/CrOS/u, "Chrome OS"],
    [/Linux/u, "Linux"],
];

/** The browser's client as its keystore stores it. */
export const ClientRecord = schema.object({
    /** The client's identifier, the principal its local objects partition by. */
    id: schema.identifier("client"),
    /** The ECDSA P-256 key pair, its private half unextractable. */
    keys: schema.object({
        /** The private half, which signs proofs. */
        privateKey: schema.instanceof(CryptoKey),
        /** The public half. */
        publicKey: schema.instanceof(CryptoKey),
    }),
    /** The public half as a JSON Web Key. */
    publicKey: PublicKey,
    /** Whether a person's sign-in registered the client and its key. */
    isRegistered: schema.boolean(),
});
/** The browser's client as its keystore stores it. */
export type ClientRecord = schema.Infer<typeof ClientRecord>;

/** Where a browser stores its client across visits. */
export interface ClientStore {
    /** Read the stored client, absent before the first visit. */
    readonly load: () => Promise<ClientRecord | undefined>;
    /** Create a client unless another tab created one first, answering the one stored. */
    readonly createIfAbsent: (created: ClientRecord) => Promise<ClientRecord>;
    /** Replace the stored client. */
    readonly save: (record: ClientRecord) => Promise<void>;
    /** Forget the client. */
    readonly clear: () => Promise<void>;
}

/** The browser profile a page runs in, as a client. */
export class BrowserClient {
    /** Where the client is stored. */
    readonly #store: ClientStore;
    /** The host path below which the host registers and binds the client. */
    readonly #path: string;
    /** The origin whose host registers and binds the client. */
    readonly #origin: string;
    /** The fetch reaching the host. */
    readonly #fetch: Fetch;

    /** Read a browser's client from a store and reach its host at a path of an origin. */
    constructor(options: {
        /** The host path below which the host registers and binds the client. */
        readonly path: string;
        /** Where the client is stored, IndexedDB by default. */
        readonly store?: ClientStore;
        /** The origin whose host registers and binds the client, the page's by default. */
        readonly origin?: string;
        /** The fetch reaching the host, the page's by default. */
        readonly fetch?: Fetch;
    }) {
        // set the store, the host's path and origin, and the fetch reaching it
        this.#path = options.path;
        this.#store = options.store ?? BrowserClient.indexedStore();
        this.#origin = options.origin ?? location.origin;
        this.#fetch = options.fetch ?? ((request) => fetch(request));
    }

    /** Describe a browser by its User-Agent Client Hints, else its user agent. */
    static describe(
        navigator: Pick<Navigator, "userAgent"> & { readonly userAgentData?: UserAgentData },
    ): Omit<ClientRegistration, "id"> {
        // read the leading brand and the platform from the hints
        const hints = navigator.userAgentData;
        const hinted = hints?.brands.find(
            (entry) => !/not.?a.?brand|chromium/iu.test(entry.brand),
        )?.brand;

        // read them from the user agent of a browser without hints
        const agent = navigator.userAgent;
        const brand = hinted ?? USER_AGENT_BRANDS.find(([pattern]) => pattern.test(agent))?.[1];
        const platform =
            (hints === undefined ? undefined : Platform.ofHint(hints.platform)) ??
            USER_AGENT_PLATFORMS.find(([pattern]) => pattern.test(agent))?.[1];
        const name = [brand ?? "Browser", platform].filter(Boolean).join(" on ");

        return { name, ...(platform === undefined ? {} : { platform }) };
    }

    /** Store the client in the browser's IndexedDB. */
    static indexedStore(): ClientStore {
        return {
            load: async () => {
                // read the record, refusing one of another shape
                const read: unknown = await transact("readonly", (store, settle) => {
                    const request = store.get(RECORD);
                    request.addEventListener("success", () => settle(request.result));
                });

                return read === undefined ? undefined : ClientRecord.parse(read);
            },
            createIfAbsent: async (created) => {
                // store the created record unless one exists
                const record: unknown = await transact("readwrite", (store, settle) => {
                    const read = store.get(RECORD);
                    read.addEventListener("success", () => {
                        const existing: unknown = read.result;
                        if (existing === undefined) {
                            store.put(created, RECORD);
                        }
                        settle(existing ?? created);
                    });
                });

                // refuse a stored record of another shape
                return ClientRecord.parse(record);
            },
            save: async (record) => {
                await transact("readwrite", (store, settle) => {
                    store.put(record, RECORD);
                    settle(undefined);
                });
            },
            clear: async () => {
                await transact("readwrite", (store, settle) => {
                    store.delete(RECORD);
                    settle(undefined);
                });
            },
        };
    }

    /** Read this browser's client as the principal its local objects partition by. */
    async subject(): Promise<Subject> {
        const record = await this.#record();

        return principal.client.reference(Scope.universe.id, record.id);
    }

    /** Bind the page's credential to this browser's client. */
    async bind(registration: Omit<ClientRegistration, "id">): Promise<Identifier<"client">> {
        // bind the client a sign-in registered
        const bound = await this.#bind(await this.#registered(registration));
        if (bound !== undefined) {
            return bound;
        }

        // take a new identity once the person removed the client, and register it
        await this.#store.clear();
        const again = await this.#bind(await this.#registered(registration));
        if (again === undefined) {
            throw new TypeError("the host removed the client it just registered");
        }

        return again;
    }

    /** Read the stored client, generating its identifier and key pair on the first visit. */
    async #record(): Promise<ClientRecord> {
        // read the client an earlier visit stored
        const record = await this.#store.load();
        if (record !== undefined) {
            return record;
        }

        // generate a key pair whose private half stays in WebCrypto, stored unless another tab stored one
        const keys = await crypto.subtle.generateKey(
            { name: "ECDSA", namedCurve: "P-256" },
            false,
            ["sign", "verify"],
        );
        const publicKey = await PublicKey.of(keys.publicKey);

        return this.#store.createIfAbsent({
            id: Identifier.create("client"),
            keys,
            publicKey,
            isRegistered: false,
        });
    }

    /** Read the stored client, registering it and its key once. */
    async #registered(registration: Omit<ClientRegistration, "id">): Promise<ClientRecord> {
        // save the registered client
        const record = await this.#record();
        if (record.isRegistered) {
            return record;
        }

        // register the client under its own identifier, then its key with a proof of possession
        const { client } = KeyRegistration.pick({ client: true }).parse(
            await this.#post(this.#path, { ...registration, id: record.id }, CREATED),
        );
        if (client !== record.id) {
            throw new TypeError(`the host registered client ${client} for client ${record.id}`);
        }
        const proof = await Proof.sign(
            record.keys.privateKey,
            record.publicKey,
            record.id,
            Date.now(),
        );
        await this.#post(`${this.#path}/key`, { client, publicKey: record.publicKey, proof }, OK);
        const registered = { ...record, isRegistered: true };
        await this.#store.save(registered);

        return registered;
    }

    /** Bind the page's credential to a registered client. */
    async #bind(record: ClientRecord): Promise<Identifier<"client"> | undefined> {
        // prove the key signs this very request
        const url = new URL(`${this.#path}/bind`, this.#origin).href;
        const proof = await Proof.sign(
            record.keys.privateKey,
            record.publicKey,
            record.id,
            Date.now(),
            {
                method: "POST",
                url,
            },
        );
        const response = await this.#fetch(
            new Request(url, { method: "POST", headers: { [PROOF_HEADER]: proof } }),
        );

        // take a gone client as one the person removed
        if (response.status === GONE) {
            return undefined;
        }
        // refuse any other failure
        else if (!response.ok) {
            throw new TypeError(`binding the client answered ${response.status}`);
        }
        // read the bound client
        else {
            return KeyRegistration.pick({ client: true }).parse(await response.json()).client;
        }
    }

    /** Post JSON to a path of the host, requiring an expected status. */
    async #post(path: string, body: unknown, expected: number): Promise<unknown> {
        // post the body and refuse an unexpected status
        const response = await this.#fetch(
            new Request(new URL(path, this.#origin), {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify(body),
            }),
        );
        if (response.status !== expected) {
            throw new TypeError(`${path} answered ${response.status}: ${await response.text()}`);
        }

        return response.json();
    }
}

/** The User-Agent Client Hints a Chromium browser exposes to script. */
export interface UserAgentData {
    /** The browser's brands, such as Google Chrome. */
    readonly brands: readonly { readonly brand: string; readonly version: string }[];
    /** The operating system, such as macOS. */
    readonly platform: string;
}

/** Run one transaction on the client's object store. */
async function transact<Value>(
    mode: IDBTransactionMode,
    run: (store: IDBObjectStore, settle: (value: Value) => void) => void,
): Promise<Value> {
    // open the database with its store
    const opening = indexedDB.open(DATABASE, 1);
    opening.addEventListener("upgradeneeded", () => opening.result.createObjectStore(DATABASE));
    const database = await requested(opening);

    // run the requests and settle once the transaction commits
    try {
        return await new Promise<Value>((resolve, reject) => {
            // open the transaction, refusing its failure
            const transaction = database.transaction(DATABASE, mode);
            transaction.addEventListener("error", () => reject(failure(transaction.error)));
            transaction.addEventListener("abort", () => reject(failure(transaction.error)));

            // run the requests, settling with their value once the transaction commits
            let settled: { readonly value: Value } | undefined;
            run(transaction.objectStore(DATABASE), (value) => {
                settled = { value };
            });
            transaction.addEventListener("complete", () => {
                // refuse a transaction that settled no value
                if (settled === undefined) {
                    reject(new TypeError("the client's transaction settled no value"));
                }
                // answer the settled value
                else {
                    resolve(settled.value);
                }
            });
        });
    } finally {
        database.close();
    }
}

/** Settle an IndexedDB request with its result or error. */
function requested<Value>(request: IDBRequest<Value>): Promise<Value> {
    return new Promise((resolve, reject) => {
        request.addEventListener("success", () => resolve(request.result));
        request.addEventListener("error", () => reject(failure(request.error)));
    });
}

/** Read the error an IndexedDB request or transaction failed with. */
function failure(error: DOMException | null): Error {
    return error ?? new TypeError("IndexedDB failed without an error");
}

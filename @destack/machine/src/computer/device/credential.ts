import { TokenCache } from "@destack/access/token";
import type { Login, LoginCredential } from "@destack/account/native";
import { CallError, PackageMount } from "@destack/object";
import { schema } from "@destack/schema";
import machinePackage from "../../package.ts";
import type { Loopback } from "../process/index.ts";
import { MachineError } from "../../error/index.ts";

/** The path below which the device forwards the machine owner's calls as a signed-in person. */
export const PEOPLE_PATH = "/people";

/** The credential of the people signed in on this computer, kept by its machine. */
export class MachineCredential implements LoginCredential {
    /** The owner's loopback connection to the machine. */
    readonly #loopback: Loopback;
    /** The access tokens of each login, by login. */
    readonly #tokens = new Map<string, TokenCache>();

    /** Ask the machine behind a loopback connection for logins' tokens and proofs. */
    constructor(loopback: Loopback) {
        this.#loopback = loopback;
    }

    /** Read the cache of a login's access tokens. */
    tokens(row: Login["row"]): TokenCache {
        // reuse the login's cache, else start it
        const known = this.#tokens.get(row.id);
        if (known !== undefined) {
            return known;
        }
        const tokens = new TokenCache();
        this.#tokens.set(row.id, tokens);

        return tokens;
    }

    /** Ask the machine for a login's account token. */
    async issue(person: Login): Promise<string> {
        const { token } = schema
            .object({ token: schema.string() })
            .parse(await this.#read(person.row.id, "token", "GET"));

        return token;
    }

    /** Ask the machine to forget a signed-out login: its refresh token and its login here. */
    async forget(row: Login["row"]): Promise<void> {
        await this.#read(row.id, "forget", "POST");
        this.#tokens.delete(row.id);
    }

    /** Ask the machine to prove a login's client. */
    async prove(person: Login): Promise<string> {
        const { proof } = schema
            .object({ proof: schema.string() })
            .parse(await this.#read(person.row.id, "proof", "GET"));

        return proof;
    }

    /** Read what the machine answers below a login's path. */
    async #read(login: string, path: string, method: "GET" | "POST"): Promise<unknown> {
        // call the machine as its owner
        const url = MachineCredential.url(login, path);
        const response = await this.#loopback.fetch(new Request(url, { method }));
        const body: unknown = await response.json();
        if (!response.ok) {
            throw (
                CallError.fromJson(body) ??
                MachineError.of("MACHINE_ANSWER_INVALID", `the machine answered ${response.status}`)
            );
        }

        return body;
    }

    /** Build the loopback URL of a path below a login. */
    static url(login: string, path: string): string {
        return `http://127.0.0.1${PackageMount.path(machinePackage.id)}${PEOPLE_PATH}/${login}/${path}`;
    }
}

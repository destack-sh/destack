import { spawnSync } from "node:child_process";
import { randomBytes, randomInt } from "node:crypto";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

/** PIV credentials stored together in one restricted Bitwarden item. */
export interface HardwareCredential {
    /** Bitwarden item identifier. */
    id: string;
    /** PIN required for signing. */
    pin: string;
    /** Recovery code for unblocking the PIN. */
    puk: string;
    /** AES-256 management key encoded as hexadecimal. */
    management: string;
}

/** Store hardware credentials without displaying them or passing them in arguments. */
export class Bitwarden {
    /** Retrieve a saved recovery identity so interrupted backups can be retried. */
    static readRecovery(name: string): string | undefined {
        // select the saved item identifier or its original exact name
        const item = this.reference(name);
        const items = item
            ? [this.run<BitwardenItem>(["get", "item", item])]
            : this.run<BitwardenItem[]>(["list", "items", "--search", name]);
        const matches = item ? items : items.filter((entry) => entry.name === name);
        if (matches.length > 1) {
            throw new Error("multiple Bitwarden records match this backup");
        }
        if (matches.length === 0) {
            return undefined;
        }

        // reject incomplete records instead of generating an unrelated replacement key
        const identity = matches[0]!.fields.find((field) => field.name === "identity")?.value;
        if (!identity) {
            throw new Error("bitwarden backup record has no recovery identity");
        }

        return identity;
    }

    /** Save an age recovery identity and verify retrieval before encrypting a backup. */
    static saveRecovery(name: string, identity: string): void {
        // refuse ambiguous recovery records instead of replacing an existing key
        const matches = this.run<BitwardenItem[]>(["list", "items", "--search", name]);
        if (matches.some((item) => item.name === name)) {
            throw new Error("a Bitwarden recovery record with this name already exists");
        }

        // send the private identity through stdin and retain it in a hidden field
        const item = {
            type: 2,
            name,
            secureNote: { type: 0 },
            notes: "age recovery identity for the matching Destack release backup. Keep an independent offline recovery copy.",
            fields: [{ name: "identity", value: identity, type: 1 }],
        };
        const created = this.run<BitwardenItem>(
            ["create", "item"],
            Buffer.from(JSON.stringify(item)).toString("base64"),
        );

        // confirm the server roundtrip before allowing backup creation
        this.run(["sync"], undefined, false);
        const actual = this.run<BitwardenItem>(["get", "item", created.id]);
        if (actual.fields.find((field) => field.name === "identity")?.value !== identity) {
            throw new Error("recovery identity verification failed; backup was not created");
        }
    }

    /** Read an exact hardware item and fail on ambiguous names. */
    static get(serial: string): HardwareCredential | undefined {
        // resolve the enrolled token's credential record
        const name = `Destack release root / YubiKey ${serial}`;
        const item = this.reference(name);
        if (item) {
            return this.decode(this.run<BitwardenItem>(["get", "item", item]));
        }
        const items = this.run<BitwardenItem[]>(["list", "items", "--search", name]);
        const matches = items.filter((item) => item.name === name);
        if (matches.length > 1) {
            throw new Error("multiple Bitwarden records match this YubiKey");
        }

        return matches.length === 1 ? this.decode(matches[0]!) : undefined;
    }

    /** Generate and verify a recoverable vault record before changing the hardware. */
    static create(serial: string): HardwareCredential {
        // save unique random values before any device mutation can occur
        const fields = [
            { name: "pin", value: randomInt(100_000_000).toString().padStart(8, "0"), type: 1 },
            { name: "puk", value: randomBytes(6).toString("base64"), type: 1 },
            { name: "management", value: randomBytes(32).toString("hex"), type: 1 },
        ];
        const item = {
            type: 2,
            name: `Destack release root / YubiKey ${serial}`,
            secureNote: { type: 0 },
            notes: `PIV slot 9c, RSA-2048, serial ${serial}. Retain this record after enrollment failures.`,
            fields,
        };
        const created = this.run<BitwardenItem>(
            ["create", "item"],
            Buffer.from(JSON.stringify(item)).toString("base64"),
        );
        const expected = this.decode(created);

        // roundtrip through the server before using the new credentials
        this.run(["sync"], undefined, false);
        const actual = this.decode(this.run<BitwardenItem>(["get", "item", expected.id]));
        if (
            actual.pin !== expected.pin ||
            actual.puk !== expected.puk ||
            actual.management !== expected.management
        ) {
            throw new Error(
                "credential verification in Bitwarden failed; hardware was not changed",
            );
        }

        return actual;
    }

    /** Confirm an unlocked session and refresh its vault records. */
    static sync(): void {
        // require the unlocked terminal session before refreshing remote records
        const status = this.run<{ status: string }>(["status"]);
        if (status.status !== "unlocked") {
            throw new Error("unlock Bitwarden and export BW_SESSION in this terminal");
        }
        this.run(["sync"], undefined, false);
    }

    /** Resolve enrolled records by stable item ID independently of their display names. */
    private static reference(name: string): string | undefined {
        // retain stable item identifiers across display-name changes
        const path =
            process.env.DESTACK_BITWARDEN_RECORDS ??
            join(homedir(), ".destack-signing", "bitwarden.json");
        let text: string;
        try {
            text = readFileSync(path, "utf8");
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code === "ENOENT") {
                return undefined;
            }
            throw error;
        }

        // select a nonempty identifier without exposing credential fields
        const records = JSON.parse(text) as Record<string, string>;
        const item = records[name];
        if (item !== undefined && (typeof item !== "string" || !item)) {
            throw new Error("invalid Bitwarden item reference");
        }

        return item;
    }

    /** Read only the expected hidden fields from a stored item. */
    private static decode(item: BitwardenItem): HardwareCredential {
        // require every enrollment credential in its documented encoding
        const fields = new Map(item.fields.map((field) => [field.name, field.value]));
        const pin = fields.get("pin");
        const puk = fields.get("puk");
        const management = fields.get("management");
        if (
            !item.id ||
            !pin ||
            !/^\d{8}$/.test(pin) ||
            !puk ||
            puk.length !== 8 ||
            !management ||
            !/^[0-9a-f]{64}$/.test(management)
        ) {
            throw new Error("invalid Bitwarden hardware credential record");
        }

        return { id: item.id, pin, puk, management };
    }

    /** Capture CLI output privately and report failures without echoing vault content. */
    private static run<Result = undefined>(
        arguments_: string[],
        input?: string,
        json = true,
    ): Result {
        if (!process.env.BW_SESSION) {
            throw new Error("export BW_SESSION in the terminal running this command");
        }
        const result = spawnSync("bw", arguments_, {
            input,
            encoding: "utf8",
            stdio: ["pipe", "pipe", "pipe"],
            timeout: 120_000,
            maxBuffer: 16 * 1024 * 1024,
        });
        if (result.error || result.status !== 0) {
            // report process failure details without exposing captured vault output
            const code = (result.error as NodeJS.ErrnoException | undefined)?.code;
            const reason = code ?? result.signal ?? `exit ${result.status}`;
            throw new Error(
                `command ${arguments_[0]} in Bitwarden failed (${reason}); check the CLI session and run bw sync directly for connection diagnostics`,
            );
        }
        if (!json) {
            return undefined as Result;
        }

        // do not expose malformed vault output through a parser's error message
        try {
            return JSON.parse(result.stdout) as Result;
        } catch {
            throw new Error("invalid Bitwarden response; vault content was not logged");
        }
    }
}

/** Fields read from a Bitwarden secure note. */
interface BitwardenItem {
    /** Stable vault identifier. */
    id: string;
    /** Exact serial-qualified record name. */
    name: string;
    /** Hidden PIV credentials. */
    fields: { name: string; value: string }[];
}

import { spawnSync } from "node:child_process";
import { randomBytes, randomInt } from "node:crypto";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { schema } from "@destack/schema";

/** Fields read from a Bitwarden secure note. */
const BitwardenItem = schema
    .object({
        /** Stable vault identifier. */
        id: schema.string(),
        /** Exact serial-qualified record name. */
        name: schema.string(),
        /** Custom fields holding hidden credentials. */
        fields: schema
            .array(
                schema.object({ name: schema.string(), value: schema.string().nullable() }).strip(),
            )
            .nullable()
            .exactOptional(),
    })
    .strip();

/** A Bitwarden item as the CLI reports it. */
type BitwardenItem = schema.Infer<typeof BitwardenItem>;

/** Items listed by a Bitwarden search. */
const BitwardenItems = schema.array(BitwardenItem);

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
export const Bitwarden = {
    readRecovery,
    saveRecovery,
    get,
    create,
    sync,
};

/** Retrieve a saved recovery identity so interrupted backups can be retried. */
function readRecovery(name: string): string | undefined {
    // select the saved item or the one item with this exact name
    const match = find(name);
    if (match === undefined) {
        return undefined;
    }

    // reject incomplete records instead of generating an unrelated replacement key
    const identity = match.fields?.find((field) => field.name === "identity")?.value ?? "";
    if (identity === "") {
        throw new Error("bitwarden backup record has no recovery identity");
    }

    return identity;
}

/** Save an age recovery identity and verify retrieval before encrypting a backup. */
function saveRecovery(name: string, identity: string): void {
    // refuse ambiguous recovery records instead of replacing an existing key
    const matches = read(["list", "items", "--search", name], BitwardenItems);
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
    const created = read(
        ["create", "item"],
        BitwardenItem,
        Buffer.from(JSON.stringify(item)).toString("base64"),
    );

    // confirm the server roundtrip before allowing backup creation
    run(["sync"]);
    const actual = read(["get", "item", created.id], BitwardenItem);
    if (actual.fields?.find((field) => field.name === "identity")?.value !== identity) {
        throw new Error("recovery identity verification failed; backup was not created");
    }
}

/** Read an exact hardware item and fail on ambiguous names. */
function get(serial: string): HardwareCredential | undefined {
    const match = find(hardwareName(serial));

    return match === undefined ? undefined : decode(match);
}

/** Generate and verify a recoverable vault record before changing the hardware. */
function create(serial: string): HardwareCredential {
    // save unique random values before any device mutation can occur
    const fields = [
        { name: "pin", value: randomInt(100_000_000).toString().padStart(8, "0"), type: 1 },
        { name: "puk", value: randomBytes(6).toString("base64"), type: 1 },
        { name: "management", value: randomBytes(32).toString("hex"), type: 1 },
    ];
    const item = {
        type: 2,
        name: hardwareName(serial),
        secureNote: { type: 0 },
        notes: `PIV slot 9c, RSA-2048, serial ${serial}. Retain this record after enrollment failures.`,
        fields,
    };
    const created = read(
        ["create", "item"],
        BitwardenItem,
        Buffer.from(JSON.stringify(item)).toString("base64"),
    );
    const expected = decode(created);

    // roundtrip through the server before using the new credentials
    run(["sync"]);
    const actual = decode(read(["get", "item", expected.id], BitwardenItem));
    if (
        actual.pin !== expected.pin ||
        actual.puk !== expected.puk ||
        actual.management !== expected.management
    ) {
        throw new Error("credential verification in Bitwarden failed; hardware was not changed");
    }

    return actual;
}

/** Confirm an unlocked session and refresh its vault records. */
function sync(): void {
    // require the unlocked terminal session before refreshing remote records
    const status = read(["status"], schema.object({ status: schema.string() }).strip());
    if (status.status !== "unlocked") {
        throw new Error("unlock Bitwarden and export BW_SESSION in this terminal");
    }
    run(["sync"]);
}

/** Find the item with an exact name, by its saved identifier or else by a search. */
function find(name: string): BitwardenItem | undefined {
    // read the item by the identifier saved at enrollment
    const item = reference(name);
    if (item !== undefined) {
        return read(["get", "item", item], BitwardenItem);
    }

    // search by name and refuse ambiguous matches
    const items = read(["list", "items", "--search", name], BitwardenItems);
    const matches = items.filter((entry) => entry.name === name);
    if (matches.length > 1) {
        throw new Error(`multiple Bitwarden records are named ${name}`);
    }

    return matches[0];
}

/** Name the vault item holding one YubiKey's credentials. */
function hardwareName(serial: string): string {
    return `Destack release root / YubiKey ${serial}`;
}

/** Resolve enrolled records by stable item ID independently of their display names. */
function reference(name: string): string | undefined {
    // retain stable item identifiers across display-name changes
    const path =
        process.env["DESTACK_BITWARDEN_RECORDS"] ??
        join(homedir(), ".destack-signing", "bitwarden.json");
    let text: string;
    try {
        text = readFileSync(path, "utf8");
    } catch (error) {
        if (error instanceof Error && "code" in error && error.code === "ENOENT") {
            return undefined;
        }
        throw error;
    }

    // select a nonempty identifier without exposing credential fields
    const records = schema.record(schema.string(), schema.string()).parse(JSON.parse(text));
    const item = records[name];
    if (item === "") {
        throw new Error("invalid Bitwarden item reference");
    }

    return item;
}

/** Read only the expected hidden fields from a stored item. */
function decode(item: BitwardenItem): HardwareCredential {
    // require every enrollment credential in its documented encoding
    const fields = new Map(item.fields?.map((field) => [field.name, field.value]));
    const pin = fields.get("pin");
    const puk = fields.get("puk");
    const management = fields.get("management");
    if (
        item.id === "" ||
        typeof pin !== "string" ||
        !/^\d{8}$/u.test(pin) ||
        typeof puk !== "string" ||
        puk.length !== 8 ||
        typeof management !== "string" ||
        !/^[0-9a-f]{64}$/u.test(management)
    ) {
        throw new Error("invalid Bitwarden hardware credential record");
    }

    return { id: item.id, pin, puk, management };
}

/** Capture CLI output privately and report failures without echoing vault content. */
function run(arguments_: string[], input?: string): string {
    // require the unlocked terminal session before invoking the CLI
    const session = process.env["BW_SESSION"];
    if (session === undefined || session === "") {
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
        const code =
            result.error && "code" in result.error && typeof result.error.code === "string"
                ? result.error.code
                : undefined;
        const reason = code ?? result.signal ?? `exit ${result.status}`;
        throw new Error(
            `command ${arguments_[0]} in Bitwarden failed (${reason}); check the CLI session and run bw sync directly for connection diagnostics`,
        );
    }

    return result.stdout;
}

/** Run a CLI command and validate its JSON response without echoing vault content. */
function read<Result>(
    arguments_: string[],
    response: schema.Schema<Result>,
    input?: string,
): Result {
    // do not expose malformed vault output through a parser's error message
    const output = run(arguments_, input);
    try {
        return response.parse(JSON.parse(output));
    } catch {
        throw new Error("invalid Bitwarden response; vault content was not logged");
    }
}

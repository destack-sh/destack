import { join } from "node:path";
import { type Environment, FileSystemError, Random } from "@destack/environment";
import { schema } from "@destack/schema";
import { communicate } from "../distribution/command.ts";

/** How long one Bitwarden CLI command may run, in milliseconds. */
const COMMAND_TIMEOUT_MILLISECONDS = 120_000;

/** The PINs a new hardware record draws from: every eight-digit number. */
const PIN_RANGE = 100_000_000;

/** Fields read from a Bitwarden secure note. */
const BitwardenItem = schema
    .object({
        /** Stable vault identifier. */
        id: schema.string(),
        /** Exact serial-qualified record name. */
        name: schema.string(),
        /** Custom fields with hidden credentials. */
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
export class Bitwarden {
    /** The host the Bitwarden CLI runs on. */
    readonly environment: Environment<"files" | "process" | "host">;

    /** Use the Bitwarden CLI of a host. */
    constructor(environment: Environment<"files" | "process" | "host">) {
        this.environment = environment;
    }

    /** Retrieve a saved recovery identity so interrupted backups can be retried. */
    async readRecovery(name: string): Promise<string | undefined> {
        // select the saved item or the one item with this exact name
        const match = await this.#find(name);
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
    async saveRecovery(name: string, identity: string): Promise<void> {
        // refuse ambiguous recovery records instead of replacing an existing key
        const matches = await this.#read(["list", "items", "--search", name], BitwardenItems);
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
        const created = await this.#read(
            ["create", "item"],
            BitwardenItem,
            Buffer.from(JSON.stringify(item)).toString("base64"),
        );

        // confirm the server roundtrip before allowing backup creation
        await this.#run(["sync"]);
        const actual = await this.#read(["get", "item", created.id], BitwardenItem);
        if (actual.fields?.find((field) => field.name === "identity")?.value !== identity) {
            throw new Error("recovery identity verification failed; backup was not created");
        }
    }

    /** Read an exact hardware item and fail on ambiguous names. */
    async get(serial: string): Promise<HardwareCredential | undefined> {
        const match = await this.#find(hardwareName(serial));

        return match === undefined ? undefined : decode(match);
    }

    /** Generate and verify a recoverable vault record before changing the hardware. */
    async create(serial: string): Promise<HardwareCredential> {
        // save unique random values before any device mutation can occur
        const { random } = this.environment;
        const fields = [
            {
                name: "pin",
                value: Random.int(random, PIN_RANGE).toString().padStart(8, "0"),
                type: 1,
            },
            { name: "puk", value: random.bytes(6).toBase64(), type: 1 },
            { name: "management", value: random.bytes(32).toHex(), type: 1 },
        ];
        const item = {
            type: 2,
            name: hardwareName(serial),
            secureNote: { type: 0 },
            notes: `PIV slot 9c, RSA-2048, serial ${serial}. Retain this record after enrollment failures.`,
            fields,
        };
        const created = await this.#read(
            ["create", "item"],
            BitwardenItem,
            Buffer.from(JSON.stringify(item)).toString("base64"),
        );
        const expected = decode(created);

        // roundtrip through the server before using the new credentials
        await this.#run(["sync"]);
        const actual = decode(await this.#read(["get", "item", expected.id], BitwardenItem));
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
    async sync(): Promise<void> {
        // require the unlocked terminal session before refreshing remote records
        const status = await this.#read(
            ["status"],
            schema.object({ status: schema.string() }).strip(),
        );
        if (status.status !== "unlocked") {
            throw new Error("unlock Bitwarden and export BW_SESSION in this terminal");
        }
        await this.#run(["sync"]);
    }

    /** Find the item with an exact name, by its saved identifier or else by a search. */
    async #find(name: string): Promise<BitwardenItem | undefined> {
        // read the item by the identifier saved at enrollment
        const item = await this.#reference(name);
        if (item !== undefined) {
            return await this.#read(["get", "item", item], BitwardenItem);
        }

        // search by name and refuse ambiguous matches
        const items = await this.#read(["list", "items", "--search", name], BitwardenItems);
        const matches = items.filter((entry) => entry.name === name);
        if (matches.length > 1) {
            throw new Error(`multiple Bitwarden records are named '${name}'`);
        }

        return matches[0];
    }

    /** Resolve enrolled records by stable item ID independently of their display names. */
    async #reference(name: string): Promise<string | undefined> {
        // retain stable item identifiers across display-name changes
        const { files, host, process } = this.environment;
        const path =
            process.variables["DESTACK_BITWARDEN_RECORDS"] ??
            join(host.home, ".destack-signing", "bitwarden.json");
        const text = await files
            .readTextFile(path)
            .catch((error: unknown) => FileSystemError.absent(error));
        if (text === undefined) {
            return undefined;
        }

        // select a nonempty identifier without exposing credential fields
        const records = schema.record(schema.string(), schema.string()).parse(JSON.parse(text));
        const item = records[name];
        if (item === "") {
            throw new Error("invalid Bitwarden item reference");
        }

        return item;
    }

    /** Capture CLI output privately and report failures without echoing vault content. */
    async #run(arguments_: string[], input?: string): Promise<string> {
        // require the unlocked terminal session before invoking the CLI
        const session = this.environment.process.variables["BW_SESSION"];
        if (session === undefined || session === "") {
            throw new Error("export BW_SESSION in the terminal running this command");
        }
        const result = await communicate("bw", arguments_, this.environment, {
            ...(input === undefined ? {} : { input }),
            timeout: COMMAND_TIMEOUT_MILLISECONDS,
        });

        // report process failure details without exposing captured vault output
        if (result.code !== 0) {
            const reason = result.signal ?? `exit ${result.code}`;
            throw new Error(
                `command '${arguments_[0]}' in Bitwarden failed ('${reason}'); check the CLI session and run bw sync directly for connection diagnostics`,
            );
        }

        return new TextDecoder().decode(result.stdout);
    }

    /** Run a CLI command and validate its JSON response without echoing vault content. */
    async #read<Result>(
        arguments_: string[],
        response: schema.Schema<Result>,
        input?: string,
    ): Promise<Result> {
        // do not expose malformed vault output through a parser's error message
        const output = await this.#run(arguments_, input);
        try {
            return response.parse(JSON.parse(output));
        } catch {
            throw new Error("invalid Bitwarden response; vault content was not logged");
        }
    }
}

/** Read the title of the vault item with one YubiKey's credentials. */
function hardwareName(serial: string): string {
    return `Destack release root / YubiKey ${serial}`;
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

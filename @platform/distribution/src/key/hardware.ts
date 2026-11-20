import { join } from "node:path";
import type { Environment } from "@destack/environment";
import { canonicalize } from "@tufjs/canonical-json";
import { Key, type Metadata, type Root, Signature } from "@tufjs/models";
import { schema } from "@destack/schema";
import { Bitwarden } from "./bitwarden.ts";
import { invokePiv } from "./piv.ts";

/** How long the hardware may take to sign, in milliseconds: a PIN entry and a touch. */
const SIGN_TIMEOUT_MILLISECONDS = 120_000;

/** An RSA root key held in a YubiKey PIV signature slot. */
export class HardwareKey {
    /** Public key exported during enrollment. */
    readonly public: Key;
    /** Hardware serial number recorded during enrollment. */
    readonly serial: string;
    /** Absolute path to the Yubico PKCS#11 module. */
    readonly module: string;
    /** The host the signing tools run on. */
    readonly environment: Environment<"files" | "process" | "host">;

    /** Select an enrolled hardware key without accepting its PIN in application code. */
    constructor(
        key: Key,
        serial: string,
        module: string,
        environment: Environment<"files" | "process" | "host">,
    ) {
        if (key.keyType !== "rsa" || key.scheme !== "rsassa-pss-sha256" || !/^\d+$/u.test(serial)) {
            throw new Error("hardware signing requires an RSA-PSS key and YubiKey serial number");
        }
        this.public = key;
        this.serial = serial;
        this.module = module;
        this.environment = environment;
    }

    /** Sign root metadata through OpenSC, requesting PIN and touch in the terminal. */
    async sign(metadata: Metadata<Root>): Promise<void> {
        // keep PIN entry in the hardware tool and private keys on the token
        const { process } = this.environment;
        if (!process.stdin.isTerminal) {
            throw new Error("hardware signing requires an interactive terminal");
        }

        // use the terminal's unlocked vault without printing or copying the signing PIN
        const bytes = Buffer.from(canonicalize(metadata.signed.toJSON()));
        const session = process.variables["BW_SESSION"];
        const signature =
            session !== undefined && session !== ""
                ? await this.signWithVault(bytes)
                : await this.signWithPrompt(bytes);

        // add the signature under the key's id
        metadata.signatures[signature.keyID] = signature;
    }

    /** Sign with the PIN of the key's Bitwarden credentials. */
    private async signWithVault(bytes: Buffer): Promise<Signature> {
        // read the PIN from the unlocked vault
        const credential = await new Bitwarden(this.environment).get(this.serial);
        if (!credential) {
            throw new Error("missing Bitwarden credentials for the selected YubiKey");
        }

        // pass it to the PIV tool through its private pipe
        const result = await invokePiv(
            {
                command: "sign",
                serial: this.serial,
                credential: { pin: credential.pin },
                message: bytes.toString("base64"),
            },
            schema.object({ signature: schema.string() }),
            this.environment,
        );

        return new Signature({ keyID: this.public.keyID, sig: result.signature });
    }

    /** Sign through pkcs11-tool, which prompts for the PIN in the terminal. */
    private async signWithPrompt(bytes: Buffer): Promise<Signature> {
        // write the message into a private temporary directory
        const { files, process, clock } = this.environment;
        const directory = await files.makeTemporaryDirectory("destack-root-sign-");
        try {
            const input = join(directory, "root");
            const output = join(directory, "signature");
            await files.writeFile(input, bytes, { mode: 0o600, createNew: true });

            // sign with RSA-PSS in the PIV signature slot
            const child = process.spawn(
                [
                    "pkcs11-tool",
                    "--module",
                    this.module,
                    "--token-label",
                    `YubiKey PIV #${this.serial}`,
                    "--login",
                    "--sign",
                    "--id",
                    "02",
                    "--mechanism",
                    "SHA256-RSA-PKCS-PSS",
                    "--mgf",
                    "MGF1-SHA256",
                    "--salt-len",
                    "32",
                    "--input-file",
                    input,
                    "--output-file",
                    output,
                ],
                {
                    stdin: "inherit",
                    stdout: "inherit",
                    stderr: "inherit",
                    signal: clock.timeout(SIGN_TIMEOUT_MILLISECONDS),
                },
            );
            const status = await child.exited;
            if (status !== 0) {
                throw new Error(`hardware signing failed with status ${status}`);
            }

            return new Signature({
                keyID: this.public.keyID,
                sig: (await files.readFile(output)).toHex(),
            });
        } finally {
            await files.remove(directory, { recursive: true, force: true });
        }
    }
}

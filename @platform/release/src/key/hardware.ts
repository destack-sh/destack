import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Key, Signature } from "@tufjs/models";
import { Bitwarden } from "./bitwarden.ts";
import { invokePiv } from "./piv.ts";

/** An RSA root key held in a YubiKey PIV signature slot. */
export class HardwareKey {
    /** Public key exported during enrollment. */
    readonly public: Key;
    /** Hardware serial number recorded during enrollment. */
    readonly serial: string;
    /** Absolute path to the Yubico PKCS#11 module. */
    readonly module: string;

    /** Select an enrolled hardware key without accepting its PIN in application code. */
    constructor(key: Key, serial: string, module: string) {
        if (key.keyType !== "rsa" || key.scheme !== "rsassa-pss-sha256" || !/^\d+$/.test(serial)) {
            throw new Error("hardware signing requires an RSA-PSS key and YubiKey serial number");
        }
        this.public = key;
        this.serial = serial;
        this.module = module;
    }

    /** Sign canonical metadata through OpenSC, requesting PIN and touch in the terminal. */
    sign(bytes: Buffer): Signature {
        // keep PIN entry in the hardware tool and private keys on the token
        if (!process.stdin.isTTY) {
            throw new Error("hardware signing requires an interactive terminal");
        }

        // use the terminal's unlocked vault without printing or copying the signing PIN
        if (process.env.BW_SESSION) {
            const credential = Bitwarden.get(this.serial);
            if (!credential) {
                throw new Error("missing Bitwarden credentials for the selected YubiKey");
            }
            const result = invokePiv<{ signature: string }>({
                command: "sign",
                serial: this.serial,
                credential: { pin: credential.pin },
                message: bytes.toString("base64"),
            });

            return new Signature({ keyID: this.public.keyID, sig: result.signature });
        }

        // retain interactive PIN entry for custodians who do not use Bitwarden
        const directory = mkdtempSync(join(tmpdir(), "destack-root-sign-"));
        try {
            const input = join(directory, "root");
            const output = join(directory, "signature");
            writeFileSync(input, bytes, { mode: 0o600, flag: "wx" });
            const result = spawnSync(
                "pkcs11-tool",
                [
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
                { stdio: "inherit", timeout: 120_000 },
            );
            if (result.error) {
                throw result.error;
            }
            if (result.status !== 0) {
                throw new Error(`hardware signing failed with status ${result.status}`);
            }

            return new Signature({
                keyID: this.public.keyID,
                sig: readFileSync(output).toString("hex"),
            });
        } finally {
            rmSync(directory, { recursive: true, force: true });
        }
    }
}

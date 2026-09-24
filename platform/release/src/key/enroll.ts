import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { isAbsolute, join } from "node:path";
import { Bitwarden } from "./bitwarden.ts";
import { invokePiv } from "./piv.ts";

/** Public configuration read before any credential or key changes. */
interface EnrollmentStatus {
    /** Whether the signature key location has no private key. */
    empty: boolean;
    /** Whether the initial PIN remains configured. */
    defaultPin: boolean;
    /** Whether the initial PUK remains configured. */
    defaultPuk: boolean;
    /** Whether the initial management key remains configured. */
    defaultManagement: boolean;
}

/** Enroll one YubiKey after saving and verifying its credentials in Bitwarden. */
async function enroll(): Promise<void> {
    // require the user's terminal session and exact hardware selection
    const [serial, directory, ...extra] = process.argv.slice(2);
    if (!serial || !/^\d+$/.test(serial) || !directory || !isAbsolute(directory) || extra.length) {
        throw new Error("usage: enroll.ts <serial> <absolute-public-directory>");
    }
    if (!process.stdin.isTTY) {
        throw new Error("enrollment requires an interactive terminal");
    }
    Bitwarden.sync();
    const status = invokePiv<EnrollmentStatus>({ command: "inspect", serial });
    const initial =
        status.empty && status.defaultPin && status.defaultPuk && status.defaultManagement;

    // reserve public recovery records before the first hardware mutation
    await mkdir(directory, { recursive: true, mode: 0o700 });
    const recordPath = join(directory, "enrollment.json");
    const recordFile = Bun.file(recordPath);
    const record = (await recordFile.exists())
        ? JSON.parse(await readFile(recordPath, "utf8"))
        : undefined;
    if (!record && (await readdir(directory)).length !== 0) {
        throw new Error("use an empty public enrollment directory");
    }
    if (record && record.serial !== serial) {
        throw new Error("enrollment directory belongs to a different YubiKey");
    }
    if (await Bun.file(join(directory, "device.json")).exists()) {
        throw new Error("this enrollment is complete; run the hardware rehearsal");
    }
    if (!initial && !record) {
        throw new Error("non-default hardware requires its original enrollment record");
    }

    // reuse a saved record after interruption and never replace known credentials
    let credential = Bitwarden.get(serial);
    if (!credential && record) {
        throw new Error("restore the recorded Bitwarden item before continuing enrollment");
    }
    if (!credential) {
        credential = Bitwarden.create(serial);
    }
    if (record && record.item !== credential.id) {
        throw new Error("vault item differs from the recorded enrollment");
    }
    if (!record) {
        await writeFile(
            recordPath,
            JSON.stringify({ serial, item: credential.id }, null, 4) + "\n",
            { flag: "wx" },
        );
    }

    // touch is the only hardware input; credentials travel through a private pipe
    console.log(
        `credentials verified in Bitwarden for YubiKey ${serial}; touch the key whenever it flashes`,
    );
    const result = invokePiv<{
        public: string;
        certificate: string;
        attestation: string;
        issuer: string;
    }>({
        command: "enroll",
        serial,
        credential,
    });
    for (const [name, value] of [
        ["root.pem", result.public],
        ["certificate.pem", result.certificate],
        ["attestation.pem", result.attestation],
        ["issuer.pem", result.issuer],
    ] as const) {
        await writeFile(join(directory, name), value, { flag: "w", mode: 0o644 });
    }

    // mark completion after saving all public records
    await writeFile(
        join(directory, "device.json"),
        JSON.stringify(
            {
                serial,
                slot: "9c",
                algorithm: "RSA2048",
                pinPolicy: "always",
                touchPolicy: "always",
                item: credential.id,
            },
            null,
            4,
        ) + "\n",
        { flag: "wx", mode: 0o644 },
    );
    console.log(`enrollment complete; public records: ${directory}`);
}

await enroll();

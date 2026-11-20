import { isAbsolute, join } from "node:path";
import type { Environment, HostCapability } from "@destack/environment";
import { Bitwarden, type HardwareCredential } from "./bitwarden.ts";
import { invokePiv } from "./piv.ts";
import { schema } from "@destack/schema";
import { print } from "../distribution/output.ts";

/** Public configuration read before any credential or key changes. */
const EnrollmentStatus = schema.object({
    /** Whether the signature key location has no private key. */
    empty: schema.boolean(),
    /** Whether the initial PIN remains configured. */
    defaultPin: schema.boolean(),
    /** Whether the initial PUK remains configured. */
    defaultPuk: schema.boolean(),
    /** Whether the initial management key remains configured. */
    defaultManagement: schema.boolean(),
});

/** Public records written by an enrollment. */
const EnrollmentRecord = schema
    .object({
        /** Hardware serial number. */
        serial: schema.string(),
        /** Bitwarden item identifier. */
        item: schema.string(),
    })
    .strip();

/** Public records written by an enrollment. */
type EnrollmentRecord = schema.Infer<typeof EnrollmentRecord>;

/** Public PIV records exported by a completed enrollment. */
const EnrollmentResult = schema.object({
    /** Public key in PEM encoding. */
    public: schema.string(),
    /** Self-signed certificate in PEM encoding. */
    certificate: schema.string(),
    /** Attestation certificate in PEM encoding. */
    attestation: schema.string(),
    /** Attestation issuer certificate in PEM encoding. */
    issuer: schema.string(),
});

/** Public PIV records exported by a completed enrollment. */
type EnrollmentResult = schema.Infer<typeof EnrollmentResult>;

/** Enroll one YubiKey after saving and verifying its credentials in Bitwarden. */
export async function enrollKey(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // require the user's terminal session and exact hardware selection
    const [serial, directory, ...extra] = arguments_;
    if (
        serial === undefined ||
        !/^\d+$/u.test(serial) ||
        directory === undefined ||
        !isAbsolute(directory) ||
        extra.length > 0
    ) {
        throw new Error("usage: key enroll <serial> <absolute-public-directory>");
    }
    if (!environment.process.stdin.isTerminal) {
        throw new Error("enrollment requires an interactive terminal");
    }
    const bitwarden = new Bitwarden(environment);
    await bitwarden.sync();
    const status = await invokePiv({ command: "inspect", serial }, EnrollmentStatus, environment);
    const isInitial =
        status.empty && status.defaultPin && status.defaultPuk && status.defaultManagement;

    // reserve public recovery records and credentials before the first hardware mutation
    await environment.files.mkdir(directory, { recursive: true, mode: 0o700 });
    const record = await readRecord(serial, directory, isInitial, environment);
    const credential = await saveCredential(serial, directory, record, bitwarden);

    // ask only for touch and pass credentials through a private pipe
    print(
        `credentials verified in Bitwarden for YubiKey ${serial}; touch the key whenever it flashes`,
        environment,
    );
    const result = await invokePiv(
        { command: "enroll", serial, credential },
        EnrollmentResult,
        environment,
    );
    await writePublicRecords(directory, result, environment);
    await writeDevice(serial, directory, credential.id, environment);
    print(`enrollment complete; public records: ${directory}`, environment);
}

/** Read the record of an interrupted enrollment and refuse directories of another state. */
async function readRecord(
    serial: string,
    directory: string,
    isInitial: boolean,
    environment: Environment<"files">,
): Promise<EnrollmentRecord | undefined> {
    // read a saved record or require an empty directory
    const { files } = environment;
    const path = join(directory, "enrollment.json");
    const record = (await files.exists(path))
        ? EnrollmentRecord.parse(JSON.parse(await files.readTextFile(path)))
        : undefined;
    if (!record && (await files.readDirectory(directory)).length !== 0) {
        throw new Error("use an empty public enrollment directory");
    }

    // require the same incomplete enrollment of this YubiKey
    if (record && record.serial !== serial) {
        throw new Error("enrollment directory belongs to a different YubiKey");
    }
    if (await files.exists(join(directory, "device.json"))) {
        throw new Error("this enrollment is complete; run the hardware rehearsal");
    }
    if (!isInitial && !record) {
        throw new Error("non-default hardware requires its original enrollment record");
    }

    return record;
}

/** Reuse or create the YubiKey's Bitwarden credentials and record their item. */
async function saveCredential(
    serial: string,
    directory: string,
    record: EnrollmentRecord | undefined,
    bitwarden: Bitwarden,
): Promise<HardwareCredential> {
    // reuse a saved record after interruption and never replace known credentials
    let credential = await bitwarden.get(serial);
    if (!credential && record) {
        throw new Error("restore the recorded Bitwarden item before continuing enrollment");
    }
    if (!credential) {
        credential = await bitwarden.create(serial);
    }
    if (record && record.item !== credential.id) {
        throw new Error("vault item differs from the recorded enrollment");
    }

    // record the item before the hardware changes
    if (!record) {
        await bitwarden.environment.files.writeTextFile(
            join(directory, "enrollment.json"),
            JSON.stringify({ serial, item: credential.id }, null, 4) + "\n",
            { createNew: true },
        );
    }

    return credential;
}

/** Write the public key and certificates the enrollment exported. */
async function writePublicRecords(
    directory: string,
    result: EnrollmentResult,
    environment: Environment<"files">,
): Promise<void> {
    // write each PEM record
    const records = new Map([
        ["root.pem", result.public],
        ["certificate.pem", result.certificate],
        ["attestation.pem", result.attestation],
        ["issuer.pem", result.issuer],
    ]);
    for (const [name, value] of records) {
        await environment.files.writeTextFile(join(directory, name), value, { mode: 0o644 });
    }
}

/** Mark the enrollment complete by writing its device record. */
async function writeDevice(
    serial: string,
    directory: string,
    item: string,
    environment: Environment<"files">,
): Promise<void> {
    // mark completion after saving all public records
    await environment.files.writeTextFile(
        join(directory, "device.json"),
        JSON.stringify(
            {
                serial,
                slot: "9c",
                algorithm: "RSA2048",
                pinPolicy: "always",
                touchPolicy: "always",
                item,
            },
            null,
            4,
        ) + "\n",
        { createNew: true, mode: 0o644 },
    );
}

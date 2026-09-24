import { X509Certificate, createPublicKey } from "node:crypto";
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { run } from "../distribution/command.ts";

/** Verify an enrolled public key against Yubico's published PIV certificate authority. */
async function verifyAttestation(directory: string): Promise<void> {
    // verify certificate signatures and validity against the independently retained manufacturer CA
    const authority = fileURLToPath(new URL("./yubico.pem", import.meta.url));
    const attestation = join(directory, "attestation.pem");
    await run("openssl", [
        "verify",
        "-CAfile",
        authority,
        "-untrusted",
        join(directory, "issuer.pem"),
        attestation,
    ]);

    // require the attested key to be the exact public root enrolled for signing
    const certificate = new X509Certificate(await readFile(attestation));
    const enrolled = createPublicKey(await readFile(join(directory, "root.pem")));
    const actual = certificate.publicKey.export({ type: "spki", format: "der" });
    const expected = enrolled.export({ type: "spki", format: "der" });
    if (!actual.equals(expected) || certificate.subject !== "CN=YubiKey PIV Attestation 9c") {
        throw new Error("manufacturer attestation does not identify the enrolled signature key");
    }
    console.log(`verified manufacturer attestation for ${directory}`);
}

// require explicit public enrollment records without accessing hardware or credentials
if (process.argv.length < 3) {
    throw new Error("provide at least one public enrollment directory");
}
for (const directory of process.argv.slice(2)) {
    await verifyAttestation(resolve(directory));
}

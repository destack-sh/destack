import { createPublicKey, X509Certificate } from "node:crypto";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { Environment, HostCapability } from "@destack/environment";
import { run } from "../distribution/command.ts";
import { print } from "../distribution/output.ts";

/** The subject of the attestation certificate of the PIV signature slot. */
const ATTESTATION_SUBJECT = "CN=YubiKey PIV Attestation 9c";

/** Verify an enrolled public key against Yubico's published PIV certificate authority. */
async function verifyAttestation(
    directory: string,
    environment: Environment<HostCapability>,
): Promise<void> {
    // verify certificate signatures and validity against the independently retained manufacturer CA
    const authority = fileURLToPath(new URL("./yubico.pem", import.meta.url));
    const attestation = join(directory, "attestation.pem");
    await run(
        "openssl",
        ["verify", "-CAfile", authority, "-untrusted", join(directory, "issuer.pem"), attestation],
        environment,
    );

    // require the attested key to be the exact public root enrolled for signing
    const { files } = environment;
    const certificate = new X509Certificate(await files.readTextFile(attestation));
    const enrolled = createPublicKey(await files.readTextFile(join(directory, "root.pem")));
    const actual = certificate.publicKey.export({ type: "spki", format: "der" });
    const expected = enrolled.export({ type: "spki", format: "der" });
    if (!actual.equals(expected) || certificate.subject !== ATTESTATION_SUBJECT) {
        throw new Error("manufacturer attestation does not identify the enrolled signature key");
    }
    print(`verified manufacturer attestation for ${directory}`, environment);
}

/** Verify each public enrollment directory the command line lists. */
export async function verifyAttestations(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // require explicit public enrollment records without accessing hardware or credentials
    const directories = arguments_;
    if (directories.length === 0) {
        throw new Error("provide at least one public enrollment directory");
    }
    for (const directory of directories) {
        await verifyAttestation(resolve(directory), environment);
    }
}

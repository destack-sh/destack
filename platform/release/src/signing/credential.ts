import { appendFile, mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { run } from "../distribution/command.ts";

/** Prepare or remove the isolated GitHub runner's Apple signing keychain. */
async function main(): Promise<void> {
    if (process.platform !== "darwin" || process.env.GITHUB_ACTIONS !== "true") {
        throw new Error("apple credential setup requires a macOS GitHub runner");
    }
    const temporary = required("RUNNER_TEMP");
    const directory = join(temporary, "destack-apple-signing");
    const keychain = join(directory, "signing.keychain-db");
    const operation = process.argv[2];
    if (operation === "remove") {
        // remove only the task's explicit temporary credential directory
        await rm(directory, { recursive: true, force: true });

        return;
    }
    if (operation !== "prepare") {
        throw new Error("select prepare or remove");
    }

    // read all inputs before creating a keychain or writing credentials
    const certificate = required("APPLE_CERTIFICATE");
    const certificatePassword = required("APPLE_CERTIFICATE_PASSWORD");
    const identity = required("APPLE_SIGNING_IDENTITY");
    const apiKey = required("APPLE_NOTARY_KEY");
    const keyId = required("APPLE_NOTARY_KEY_ID");
    const issuer = required("APPLE_NOTARY_ISSUER");
    const environment = required("GITHUB_ENV");
    const password = crypto.getRandomValues(new Uint8Array(32)).toHex();
    await mkdir(directory, { mode: 0o700 });

    // provision the temporary signing keychain
    try {
        // import the certificate without changing the user's default keychain
        const archive = join(directory, "certificate.p12");
        const key = join(directory, "notary.p8");
        await writeFile(archive, Buffer.from(certificate, "base64"), { flag: "wx", mode: 0o600 });
        await writeFile(key, apiKey, { flag: "wx", mode: 0o600 });
        await run("security", ["create-keychain", "-p", password, keychain]);
        await run("security", ["set-keychain-settings", "-lut", "21600", keychain]);
        await run("security", ["unlock-keychain", "-p", password, keychain]);
        await run("security", [
            "import",
            archive,
            "-k",
            keychain,
            "-P",
            certificatePassword,
            "-T",
            "/usr/bin/codesign",
        ]);
        await run("security", [
            "set-key-partition-list",
            "-S",
            "apple-tool:,apple:,codesign:",
            "-s",
            "-k",
            password,
            keychain,
        ]);
        await run("xcrun", [
            "notarytool",
            "store-credentials",
            "destack-release",
            "--keychain",
            keychain,
            "--key",
            key,
            "--key-id",
            keyId,
            "--issuer",
            issuer,
        ]);

        // retain only the keychain for the signing steps
        await rm(archive);
        await rm(key);
        await appendFile(
            environment,
            `APPLE_KEYCHAIN=${keychain}\nAPPLE_SIGNING_IDENTITY=${identity}\nAPPLE_NOTARY_PROFILE=destack-release\n`,
        );
    } catch (error) {
        await rm(directory, { recursive: true, force: true });
        throw error;
    }
}

/** Require a credential without including its value in errors. */
function required(name: string): string {
    const value = process.env[name];
    if (
        !value ||
        (/[\r\n]/.test(value) && name !== "APPLE_NOTARY_KEY" && name !== "APPLE_CERTIFICATE")
    ) {
        throw new Error(`${name} is missing or invalid`);
    }

    return value;
}

await main();

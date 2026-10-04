import { appendFile, mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { schema } from "@destack/schema";
import { run } from "../distribution/command.ts";

/** Prepare or remove the isolated GitHub runner's Apple signing keychain. */
async function main(): Promise<void> {
    if (process.platform !== "darwin" || process.env["GITHUB_ACTIONS"] !== "true") {
        throw new Error("apple credential setup requires a macOS GitHub runner");
    }
    const temporary = required("RUNNER_TEMP");
    const directory = join(temporary, "destack-apple-signing");
    const keychain = join(directory, "signing.keychain-db");
    const operation = process.argv[2];

    // remove only the task's explicit temporary credential directory
    if (operation === "remove") {
        await remove(directory, keychain);
    }
    // provision the temporary signing keychain
    else if (operation === "prepare") {
        await prepare(directory, keychain);
    }
    // refuse other operations
    else {
        throw new Error("select prepare or remove");
    }
}

/** Provision a signing keychain with the certificate and notary profile, removing it on failure. */
async function prepare(directory: string, keychain: string): Promise<void> {
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
    try {
        // write the private inputs into the credential directory
        const archive = join(directory, "certificate.p12");
        const key = join(directory, "notary.p8");
        await writeFile(archive, Buffer.from(certificate, "base64"), { flag: "wx", mode: 0o600 });
        await writeFile(key, apiKey, { flag: "wx", mode: 0o600 });

        // build the keychain and store the notary profile in it
        await createKeychain(keychain, password, archive, certificatePassword);
        await storeNotaryProfile(keychain, key, keyId, issuer);

        // retain only the keychain for the signing steps
        await rm(archive);
        await rm(key);
        await appendFile(
            environment,
            `APPLE_KEYCHAIN=${keychain}\nAPPLE_SIGNING_IDENTITY=${identity}\nAPPLE_NOTARY_PROFILE=destack-release\n`,
        );
    } catch (error) {
        await remove(directory, keychain);
        throw error;
    }
}

/** Create an unlocked keychain holding the certificate and add it to the search order. */
async function createKeychain(
    keychain: string,
    password: string,
    archive: string,
    certificatePassword: string,
): Promise<void> {
    // import the certificate without changing the user's default keychain
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

    // include the signing identity and its intermediate in certificate-chain searches
    const search = await readKeychains();
    await run("security", [
        "list-keychains",
        "-d",
        "user",
        "-s",
        ...new Set([...search, keychain]),
    ]);
}

/** Store the App Store Connect key as the `destack-release` notary profile. */
async function storeNotaryProfile(
    keychain: string,
    key: string,
    keyId: string,
    issuer: string,
): Promise<void> {
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
}

/** Read the user's configured signing keychain search order. */
async function readKeychains(): Promise<string[]> {
    // capture public keychain paths while preserving tool failures
    const process = Bun.spawn(["security", "list-keychains", "-d", "user"], {
        stdout: "pipe",
        stderr: "inherit",
        timeout: 10_000,
    });
    const output = await new Response(process.stdout).text();
    if ((await process.exited) !== 0) {
        throw new Error("could not read the keychain search list");
    }

    return output
        .trim()
        .split("\n")
        .filter(Boolean)
        .map((line) => schema.string().parse(JSON.parse(line.trim())));
}

/** Remove the runner's signing keychain and preserve other search-list entries. */
async function remove(directory: string, keychain: string): Promise<void> {
    // remove only this job's keychain from the shared search order
    const search = await readKeychains();
    await run("security", [
        "list-keychains",
        "-d",
        "user",
        "-s",
        ...search.filter((path) => path !== keychain),
    ]);

    // delete the keychain through Security before removing the credential directory
    if (await Bun.file(keychain).exists()) {
        await run("security", ["delete-keychain", keychain]);
    }
    await rm(directory, { recursive: true, force: true });
}

/** Require a credential without including its value in errors. */
function required(name: string): string {
    const value = process.env[name];
    if (
        value === undefined ||
        value === "" ||
        (/[\r\n]/u.test(value) && name !== "APPLE_NOTARY_KEY" && name !== "APPLE_CERTIFICATE")
    ) {
        throw new Error(`${name} is missing or invalid`);
    }

    return value;
}

await main();

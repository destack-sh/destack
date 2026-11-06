import { join } from "node:path";
import type { Environment, HostCapability } from "@destack/environment";
import { schema } from "@destack/schema";
import { run } from "../distribution/command.ts";

/** How long reading the keychain search list may take, in milliseconds. */
const LIST_TIMEOUT_MILLISECONDS = 10_000;

/** Prepare or remove the isolated GitHub runner's Apple signing keychain. */
export async function provisionAppleCredential(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // require a macOS GitHub runner
    const isRunner = environment.process.variables["GITHUB_ACTIONS"] === "true";
    if (environment.host.os !== "darwin" || !isRunner) {
        throw new Error("signing credential setup requires a macOS GitHub runner");
    }
    const temporary = required("RUNNER_TEMP", environment);
    const directory = join(temporary, "destack-apple-signing");
    const keychain = join(directory, "signing.keychain-db");
    const [operation] = arguments_;

    // remove only the task's explicit temporary credential directory
    if (operation === "remove") {
        await remove(directory, keychain, environment);
    }
    // provision the temporary signing keychain
    else if (operation === "prepare") {
        await prepare(directory, keychain, environment);
    }
    // refuse other operations
    else {
        throw new Error("select prepare or remove");
    }
}

/** Provision a signing keychain with the certificate and notary profile, removing it on failure. */
async function prepare(
    directory: string,
    keychain: string,
    environment: Environment<HostCapability>,
): Promise<void> {
    // read all inputs before creating a keychain or writing credentials
    const certificate = required("APPLE_CERTIFICATE", environment);
    const certificatePassword = required("APPLE_CERTIFICATE_PASSWORD", environment);
    const identity = required("APPLE_SIGNING_IDENTITY", environment);
    const apiKey = required("APPLE_NOTARY_KEY", environment);
    const keyId = required("APPLE_NOTARY_KEY_ID", environment);
    const issuer = required("APPLE_NOTARY_ISSUER", environment);
    const variables = required("GITHUB_ENV", environment);
    const password = environment.random.bytes(32).toHex();
    const { files } = environment;
    await files.mkdir(directory, { mode: 0o700 });
    try {
        // write the private inputs into the credential directory
        const archive = join(directory, "certificate.p12");
        const key = join(directory, "notary.p8");
        const exclusive = { createNew: true, mode: 0o600 };
        await files.writeFile(archive, Buffer.from(certificate, "base64"), exclusive);
        await files.writeTextFile(key, apiKey, exclusive);

        // build the keychain and store the notary profile in it
        await createKeychain(keychain, password, archive, certificatePassword, environment);
        await storeNotaryProfile(keychain, key, keyId, issuer, environment);

        // retain only the keychain for the signing steps
        await files.remove(archive);
        await files.remove(key);
        await files.writeTextFile(
            variables,
            `APPLE_KEYCHAIN=${keychain}\nAPPLE_SIGNING_IDENTITY=${identity}\nAPPLE_NOTARY_PROFILE=destack-release\n`,
            { append: true },
        );
    } catch (error) {
        await remove(directory, keychain, environment);
        throw error;
    }
}

/** Create an unlocked keychain with the certificate and add it to the search order. */
async function createKeychain(
    keychain: string,
    password: string,
    archive: string,
    certificatePassword: string,
    environment: Environment<"process">,
): Promise<void> {
    // import the certificate without changing the user's default keychain
    await run("security", ["create-keychain", "-p", password, keychain], environment);
    await run("security", ["set-keychain-settings", "-lut", "21600", keychain], environment);
    await run("security", ["unlock-keychain", "-p", password, keychain], environment);
    await run(
        "security",
        ["import", archive, "-k", keychain, "-P", certificatePassword, "-T", "/usr/bin/codesign"],
        environment,
    );
    await run(
        "security",
        [
            "set-key-partition-list",
            "-S",
            "apple-tool:,apple:,codesign:",
            "-s",
            "-k",
            password,
            keychain,
        ],
        environment,
    );

    // include the signing identity and its intermediate in certificate-chain searches
    const search = await readKeychains(environment);
    await run(
        "security",
        ["list-keychains", "-d", "user", "-s", ...new Set([...search, keychain])],
        environment,
    );
}

/** Store the App Store Connect key as the `destack-release` notary profile. */
async function storeNotaryProfile(
    keychain: string,
    key: string,
    keyId: string,
    issuer: string,
    environment: Environment<"process">,
): Promise<void> {
    await run(
        "xcrun",
        [
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
        ],
        environment,
    );
}

/** Read the user's configured signing keychain search order. */
async function readKeychains(environment: Environment<"process">): Promise<string[]> {
    // capture public keychain paths while preserving tool failures
    const child = environment.process.spawn(["security", "list-keychains", "-d", "user"], {
        stdout: "pipe",
        stderr: "inherit",
        signal: environment.clock.timeout(LIST_TIMEOUT_MILLISECONDS),
    });
    const output = await new Response(child.stdout).text();
    if ((await child.exited) !== 0) {
        throw new Error("could not read the keychain search list");
    }

    return output
        .trim()
        .split("\n")
        .filter(Boolean)
        .map((line) => schema.string().parse(JSON.parse(line.trim())));
}

/** Remove the runner's signing keychain and preserve other search-list entries. */
async function remove(
    directory: string,
    keychain: string,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // remove only this job's keychain from the shared search order
    const search = await readKeychains(environment);
    await run(
        "security",
        ["list-keychains", "-d", "user", "-s", ...search.filter((path) => path !== keychain)],
        environment,
    );

    // delete the keychain through Security before removing the credential directory
    if (await environment.files.exists(keychain)) {
        await run("security", ["delete-keychain", keychain], environment);
    }
    await environment.files.remove(directory, { recursive: true, force: true });
}

/** Require a credential without including its value in errors. */
function required(name: string, environment: Environment<"process">): string {
    const value = environment.process.variables[name];
    if (
        value === undefined ||
        value === "" ||
        (/[\r\n]/u.test(value) && name !== "APPLE_NOTARY_KEY" && name !== "APPLE_CERTIFICATE")
    ) {
        throw new Error(`${name} is missing or invalid`);
    }

    return value;
}

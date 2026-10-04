import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { SigningKey } from "@destack/update/publish";

/** The online roles a release signs with. */
const RELEASE_ROLES: readonly string[] = ["targets", "snapshot", "timestamp"];

/** The online roles a freshness renewal signs with. */
const RENEWAL_ROLES: readonly string[] = ["snapshot", "timestamp"];

/** Materialize online CI keys or remove the exact directory created for this job. */
async function main(): Promise<void> {
    // allow cleanup after a preceding step failed before creating credentials
    if (process.argv[2] === "remove") {
        await removeKeys();
    }
    // write the keys of the selected operation
    else {
        await writeKeys(process.argv[2]);
    }
}

/** Remove the key directory of this job if it exists. */
async function removeKeys(): Promise<void> {
    // skip jobs that never created the directory
    const directory = process.env["DESTACK_RELEASE_KEYS"];
    const temporary = process.env["RUNNER_TEMP"];
    if (directory === undefined || directory === "") {
        return;
    }

    // refuse any directory except a key directory directly below the runner's temporary directory
    if (
        temporary === undefined ||
        temporary === "" ||
        dirname(resolve(directory)) !== resolve(temporary) ||
        !basename(directory).startsWith("destack-release-keys-")
    ) {
        throw new Error("refusing to remove an unexpected credential directory");
    }
    await rm(directory, { recursive: true, force: true });
}

/** Write the role keys of a release or renewal into a private directory and export its path. */
async function writeKeys(operation: string | undefined): Promise<void> {
    // require the runner's private temporary directory and environment file
    const temporary = process.env["RUNNER_TEMP"];
    const environment = process.env["GITHUB_ENV"];
    if (
        temporary === undefined ||
        temporary === "" ||
        environment === undefined ||
        environment === ""
    ) {
        throw new Error("missing RUNNER_TEMP or GITHUB_ENV");
    }
    const keys = readKeys(operation);

    // expose only the private directory path to subsequent workflow steps
    const directory = await mkdtemp(join(temporary, "destack-release-keys-"));
    await writeFile(environment, `DESTACK_RELEASE_KEYS=${directory}\n`, { flag: "a" });
    for (const { role, key } of keys) {
        await writeFile(join(directory, `${role}.pem`), key.private, {
            flag: "wx",
            mode: 0o600,
        });
    }
}

/** Read the independent role keys an operation signs with from the environment. */
function readKeys(operation: string | undefined): { role: string; key: SigningKey }[] {
    // validate separate role secrets before writing private files
    if (operation !== "release" && operation !== "renew") {
        throw new Error("select release or renew credentials");
    }
    const roles = operation === "release" ? RELEASE_ROLES : RENEWAL_ROLES;
    const keys = roles.map((role) => {
        // require each selected role without echoing its secret
        const name = `DESTACK_${role.toUpperCase()}_KEY`;
        const pem = process.env[name];
        if (pem === undefined || pem === "") {
            throw new Error(`missing ${name}`);
        }

        return { role, key: new SigningKey(pem) };
    });

    // require a distinct key per role
    if (new Set(keys.map(({ key }) => key.public.keyID)).size !== roles.length) {
        throw new Error("online metadata roles require independent keys");
    }

    return keys;
}

await main();

import type { Environment, HostCapability } from "@destack/environment";
import { basename, dirname, join, resolve } from "node:path";
import { SigningKey } from "@destack/update/publish";

/** The online roles a release signs with. */
const RELEASE_ROLES: readonly string[] = ["targets", "snapshot", "timestamp"];

/** The online roles a freshness renewal signs with. */
const RENEWAL_ROLES: readonly string[] = ["snapshot", "timestamp"];

/** Materialize online CI keys or remove the exact directory created for this job. */
export async function provisionGithubCredential(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // allow cleanup after a preceding step failed before creating credentials
    const [operation] = arguments_;
    if (operation === "remove") {
        await removeKeys(environment);
    }
    // write the keys of the selected operation
    else {
        await writeKeys(operation, environment);
    }
}

/** Remove the key directory of this job if it exists. */
async function removeKeys(environment: Environment<"files" | "process">): Promise<void> {
    // skip jobs that never created the directory
    const { variables } = environment.process;
    const directory = variables["DESTACK_RELEASE_KEYS"];
    const temporary = variables["RUNNER_TEMP"];
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
    await environment.files.remove(directory, { recursive: true, force: true });
}

/** Write the role keys of a release or renewal into a private directory and export its path. */
async function writeKeys(
    operation: string | undefined,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // require the runner's private temporary directory and environment file
    const { files, process } = environment;
    const temporary = process.variables["RUNNER_TEMP"];
    const variables = process.variables["GITHUB_ENV"];
    if (
        temporary === undefined ||
        temporary === "" ||
        variables === undefined ||
        variables === ""
    ) {
        throw new Error("missing RUNNER_TEMP or GITHUB_ENV");
    }
    const keys = await readKeys(operation, environment);

    // expose only the private directory path to subsequent workflow steps
    const directory = await files.makeTemporaryDirectory("destack-release-keys-", {
        directory: temporary,
    });
    await files.writeTextFile(variables, `DESTACK_RELEASE_KEYS=${directory}\n`, { append: true });
    for (const { role, key } of keys) {
        await files.writeTextFile(join(directory, `${role}.pem`), key.private, {
            createNew: true,
            mode: 0o600,
        });
    }
}

/** Read the independent role keys an operation signs with from the environment. */
async function readKeys(
    operation: string | undefined,
    environment: Environment<"process">,
): Promise<{ role: string; key: SigningKey }[]> {
    // validate separate role secrets before writing private files
    if (operation !== "release" && operation !== "renew") {
        throw new Error("select release or renew credentials");
    }
    const roles = operation === "release" ? RELEASE_ROLES : RENEWAL_ROLES;
    const keys = await Promise.all(
        roles.map(async (role) => {
            // require each selected role without echoing its secret
            const name = `DESTACK_${role.toUpperCase()}_KEY`;
            const pem = environment.process.variables[name];
            if (pem === undefined || pem === "") {
                throw new Error(`missing ${name}`);
            }

            return { role, key: await SigningKey.import(pem, environment.crypto) };
        }),
    );

    // require a distinct key per role
    if (new Set(keys.map(({ key }) => key.public.keyID)).size !== roles.length) {
        throw new Error("online metadata roles require independent keys");
    }

    return keys;
}

import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { SigningKey } from "./key.ts";

/** Materialize online CI keys or remove the exact directory created for this job. */
async function main(): Promise<void> {
    // allow cleanup after a preceding step failed before creating credentials
    if (process.argv[2] === "remove") {
        const directory = process.env.DESTACK_RELEASE_KEYS;
        const temporary = process.env.RUNNER_TEMP;
        if (!directory) {
            return;
        }
        if (
            !temporary ||
            dirname(resolve(directory)) !== resolve(temporary) ||
            !basename(directory).startsWith("destack-release-keys-")
        ) {
            throw new Error("refusing to remove an unexpected credential directory");
        }
        await rm(directory, { recursive: true, force: true });

        return;
    }

    // require the runner's private temporary directory and environment file
    const temporary = process.env.RUNNER_TEMP;
    const environment = process.env.GITHUB_ENV;
    if (!temporary || !environment) {
        throw new Error("missing RUNNER_TEMP or GITHUB_ENV");
    }

    // validate separate role secrets before writing private files
    const operation = process.argv[2];
    if (operation !== "release" && operation !== "renew") {
        throw new Error("select release or renew credentials");
    }
    const roles =
        operation === "release"
            ? (["targets", "snapshot", "timestamp"] as const)
            : (["snapshot", "timestamp"] as const);
    const keys = roles.map((role) => {
        // require each selected role without echoing its secret
        const name = `DESTACK_${role.toUpperCase()}_KEY`;
        const pem = process.env[name];
        if (!pem) {
            throw new Error(`missing ${name}`);
        }

        return new SigningKey(pem);
    });
    if (new Set(keys.map((key) => key.public.keyID)).size !== roles.length) {
        throw new Error("online metadata roles require independent keys");
    }

    // expose only the private directory path to subsequent workflow steps
    const directory = await mkdtemp(join(temporary, "destack-release-keys-"));
    await writeFile(environment, `DESTACK_RELEASE_KEYS=${directory}\n`, { flag: "a" });
    for (const [index, role] of roles.entries()) {
        await writeFile(join(directory, `${role}.pem`), keys[index]!.private, {
            flag: "wx",
            mode: 0o600,
        });
    }
}

await main();

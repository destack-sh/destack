import { realpath } from "node:fs/promises";
import { isAbsolute } from "node:path";
import { devNull } from "node:os";
import { ServiceError } from "@destack/service/error";
import type { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import type { CheckoutRegistration } from "../service/checkout.ts";
import type { CheckoutStore } from "./store.ts";

/** Register the canonical Git root without changing repository contents. */
export async function registerCheckout(
    input: CheckoutRegistration,
    store: CheckoutStore,
    audit: AuditRecorder<DatabaseConnection>,
) {
    // require an explicit local path before inspecting the repository
    if (!isAbsolute(input.directory)) {
        throw new ServiceError("BAD_REQUEST", { message: "checkout directory must be absolute" });
    }
    const directory = await realpath(input.directory);
    const git = Bun.spawn(["git", "-C", directory, "rev-parse", "--show-toplevel"], {
        env: {
            PATH: process.env.PATH,
            SystemRoot: process.env.SystemRoot,
            GIT_CONFIG_NOSYSTEM: "1",
            GIT_CONFIG_GLOBAL: devNull,
        },
        stdout: "pipe",
        stderr: "pipe",
        timeout: 3000,
    });
    const [code, output, error] = await Promise.all([
        git.exited,
        new Response(git.stdout).text(),
        new Response(git.stderr).text(),
    ]);

    // retain Git's diagnostic without treating a non-repository as a valid checkout
    if (code !== 0) {
        throw new ServiceError("BAD_REQUEST", {
            message: `cannot register checkout: ${error.trim()}`,
        });
    }
    const root = await realpath(output.trim());

    return await store.register(root, input.repositoryId ?? null, audit);
}

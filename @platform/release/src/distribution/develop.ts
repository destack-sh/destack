import { Stage } from "@destack/update/release";
import { join } from "node:path";
import type { Environment, HostCapability } from "@destack/environment";
import { configureApplication } from "./application.ts";
import { CHECKOUT, releaseVersion } from "./distribution.ts";

/** Run the native shell with the development identity and generated icons. */
export async function developDistribution(environment: Environment<HostCapability>): Promise<void> {
    // isolate generated native configuration
    const { files, process } = environment;
    const staging = join(CHECKOUT, "dist/.build");
    await files.mkdir(staging, { recursive: true });
    const directory = await files.makeTemporaryDirectory("desktop-dev-", { directory: staging });
    const identity = Stage.of("local");
    const version = await releaseVersion(CHECKOUT, environment);

    // retain generated icons for Cargo's build and the complete interactive lifetime
    try {
        const configuration = await configureApplication(
            CHECKOUT,
            directory,
            identity,
            version,
            environment,
        );
        const child = process.spawn(["cargo", "run", "--manifest-path", "native/Cargo.toml"], {
            workingDirectory: join(CHECKOUT, "@destack/desktop"),
            variables: {
                ...process.variables,
                TAURI_CONFIG: configuration,
                DESTACK_STAGE: identity.name,
                DESTACK_UNIVERSE: process.variables["DESTACK_UNIVERSE"] ?? identity.universe,
                CARGO_TARGET_DIR: join(CHECKOUT, "dist/.native"),
            },
            stdin: "inherit",
            stdout: "inherit",
            stderr: "inherit",
        });
        const code = await child.exited;
        if (code !== 0) {
            throw new Error(`desktop exited with status ${code}`);
        }
    } finally {
        await files.remove(directory, { recursive: true, force: true });
    }
}

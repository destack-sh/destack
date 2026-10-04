import { ReleaseChannel } from "@destack/daemon/process";
import { mkdtemp, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { configureApplication } from "./application.ts";
import { version } from "./distribution.ts";

/** Repository containing the native shell. */
const ROOT = fileURLToPath(new URL("../../../../", import.meta.url));

/** Run the native shell with the development identity and generated icons. */
async function develop(): Promise<void> {
    // isolate generated native configuration
    const staging = join(ROOT, "dist/.build");
    await mkdir(staging, { recursive: true });
    const directory = await mkdtemp(join(staging, "desktop-dev-"));
    const identity = ReleaseChannel.of("dev");

    // retain generated icons for Cargo's build and the complete interactive lifetime
    try {
        const configuration = await configureApplication(ROOT, directory, identity, version);
        const child = Bun.spawn(["cargo", "run", "--manifest-path", "native/Cargo.toml"], {
            cwd: join(ROOT, "@destack/desktop"),
            env: {
                ...process.env,
                TAURI_CONFIG: configuration,
                DESTACK_RELEASE_CHANNEL: identity.name,
                DESTACK_ISSUER: process.env["DESTACK_ISSUER"] ?? identity.issuer,
                CARGO_TARGET_DIR: join(ROOT, "dist/.native"),
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
        await rm(directory, { recursive: true, force: true });
    }
}

await develop();

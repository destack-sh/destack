import { Stage } from "@destack/update/release";
import type { Environment } from "@destack/environment";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { run } from "./command.ts";

/** Generate the native identity and artwork for a distribution or interactive run. */
export async function configureApplication(
    root: string,
    directory: string,
    identity: Stage,
    version: string,
    environment: Environment<"files" | "process" | "host">,
): Promise<string> {
    // select the authored artwork for this identity
    const isMac = environment.host.os === "darwin";
    const icons = join(directory, "icons");
    await environment.files.mkdir(icons, { recursive: true });
    const tauri = join(root, "@destack/desktop/node_modules/@tauri-apps/cli/tauri.js");
    let artwork = fileURLToPath(
        import.meta.resolve(`@platform/brand/icon/icon${identity.suffix}-rounded.svg`),
    );

    // match the visual padding of native macOS applications
    if (isMac) {
        artwork = join(icons, "source.png");
        await run(
            "swift",
            [
                "-module-cache-path",
                join(directory, "swift"),
                join(root, "@platform/release/src/distribution/icon.swift"),
                fileURLToPath(
                    import.meta.resolve(`@platform/brand/icon/icon${identity.suffix}-rounded.png`),
                ),
                artwork,
            ],
            environment,
            root,
        );
    }

    // generate native icon containers and the matching Tauri configuration
    await run(
        environment.process.executable,
        ["run", tauri, "icon", artwork, "--output", icons],
        environment,
        root,
    );

    return JSON.stringify({
        productName: identity.title,
        version,
        identifier: identity.applicationIdentifier,
        bundle: {
            icon: [join(icons, "icon.png"), join(icons, isMac ? "icon.icns" : "icon.ico")],
        },
    });
}

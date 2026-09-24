import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { ReleaseIdentity } from "./identity.ts";
import { run } from "./command.ts";

/** Generate the native identity and artwork for a distribution or interactive run. */
export async function configureApplication(
    root: string,
    directory: string,
    identity: ReleaseIdentity,
    version: string,
): Promise<string> {
    // select the authored artwork for this identity
    const icons = join(directory, "icons");
    await mkdir(icons, { recursive: true });
    const tauri = join(root, "@destack/desktop/node_modules/@tauri-apps/cli/tauri.js");
    let artwork = join(root, `platform/brand/icon/icon${identity.suffix}.svg`);

    // match the visual padding of native macOS applications
    if (process.platform === "darwin") {
        artwork = join(icons, "source.png");
        await run(
            "swift",
            [
                "-module-cache-path",
                join(directory, "swift"),
                join(root, "platform/release/src/icon/macos.swift"),
                join(root, `platform/brand/icon/icon${identity.suffix}.png`),
                artwork,
            ],
            root,
        );
    }

    // generate native icon containers and the matching Tauri configuration
    await run(process.execPath, ["run", tauri, "icon", artwork, "--output", icons], root);

    return JSON.stringify({
        productName: identity.title,
        version,
        identifier: identity.applicationIdentifier,
        bundle: {
            icon: [
                join(icons, "icon.png"),
                join(icons, process.platform === "darwin" ? "icon.icns" : "icon.ico"),
            ],
        },
    });
}

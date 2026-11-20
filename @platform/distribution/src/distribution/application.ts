import { Profile } from "@destack/update/release";
import type { Environment } from "@destack/environment";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { run } from "./command.ts";

/** Generate the native identity and artwork for a distribution or interactive run. */
export async function configureApplication(
    root: string,
    directory: string,
    profile: Profile,
    version: string,
    environment: Environment<"files" | "process" | "host">,
): Promise<string> {
    // take the brand's macOS icons as built once, so every slice of a universal build carries the same bytes
    const isMac = environment.host.os === "darwin";
    if (isMac) {
        return JSON.stringify({
            productName: profile.title,
            version,
            identifier: profile.applicationIdentifier,
            bundle: {
                icon: [
                    brand(`icon${profile.suffix}-macos.png`),
                    brand(`icon${profile.suffix}.icns`),
                ],
            },
        });
    }

    // generate the native icon containers of other systems from the authored artwork
    const icons = join(directory, "icons");
    await environment.files.mkdir(icons, { recursive: true });
    const tauri = join(root, "@destack/desktop/node_modules/@tauri-apps/cli/tauri.js");
    await run(
        environment.process.executable,
        ["run", tauri, "icon", brand(`icon${profile.suffix}-rounded.svg`), "--output", icons],
        environment,
        root,
    );

    return JSON.stringify({
        productName: profile.title,
        version,
        identifier: profile.applicationIdentifier,
        bundle: {
            icon: [join(icons, "icon.png"), join(icons, "icon.ico")],
        },
    });
}

/** Resolve a file of the brand package's icon directory. */
function brand(file: string): string {
    return fileURLToPath(import.meta.resolve(`@platform/brand/icon/${file}`));
}

import { CHECKOUT } from "./distribution.ts";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Environment, HostCapability } from "@destack/environment";
import { Stage } from "@destack/update/release";
import { run } from "./command.ts";

/** The brand package's icon directory, resolved through its exports. */
const BRAND_ICONS = dirname(fileURLToPath(import.meta.resolve("@platform/brand/icon/icon.svg")));

/** Build the stable and development icons from their vector artwork. */
export async function buildIcons(environment: Environment<HostCapability>): Promise<void> {
    // prepare the output directory and isolate intermediate platform files
    const { files } = environment;
    const output = join(CHECKOUT, "dist/icon");
    const temporary = await files.makeTemporaryDirectory("destack-icon-");
    const tauri = join(CHECKOUT, "@destack/desktop/node_modules/@tauri-apps/cli/tauri.js");
    await files.mkdir(output, { recursive: true });

    // build every identity from its corresponding authored artwork
    try {
        for (const { suffix } of Stage.names.map((name) => Stage.of(name))) {
            await buildIdentity(suffix, tauri, temporary, output, environment);
        }
    } finally {
        await files.remove(temporary, { recursive: true, force: true });
    }
}

/** Build the icons of one identity, named by its artwork suffix. */
async function buildIdentity(
    suffix: string,
    tauri: string,
    temporary: string,
    output: string,
    environment: Environment<HostCapability>,
): Promise<void> {
    // generate standard raster sizes
    const { files, process } = environment;
    const name = `destack${suffix}`;
    const source = join(BRAND_ICONS, `icon${suffix}-rounded.svg`);
    const directory = join(temporary, name);
    const words = ["run", tauri, "icon", source, "--output", directory];
    await run(process.executable, words, environment, CHECKOUT);
    await files.copyFile(source, join(output, `${name}.svg`));
    await files.copyFile(join(directory, "icon.png"), join(output, `${name}.png`));

    // build the native icon container on macOS
    if (environment.host.os === "darwin") {
        await buildMacIcon(name, tauri, temporary, directory, output, environment);
    }
}

/** Pad the raster icon for macOS and generate its `.icns` container. */
async function buildMacIcon(
    name: string,
    tauri: string,
    temporary: string,
    directory: string,
    output: string,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // apply the macOS padding before generating the native icon container
    const { files, process } = environment;
    const padded = join(directory, "macos.png");
    await run(
        "swift",
        [
            "-module-cache-path",
            join(temporary, "swift"),
            join(CHECKOUT, "@platform/release/src/distribution/icon.swift"),
            join(directory, "icon.png"),
            padded,
        ],
        environment,
        CHECKOUT,
    );

    // generate the container and copy both files
    const macos = join(directory, "macos");
    const words = ["run", tauri, "icon", padded, "--output", macos];
    await run(process.executable, words, environment, CHECKOUT);
    await files.copyFile(padded, join(output, `${name}-macos.png`));
    await files.copyFile(join(macos, "icon.icns"), join(output, `${name}.icns`));
}

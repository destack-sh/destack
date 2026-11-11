import { CHECKOUT } from "./distribution.ts";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Environment, HostCapability } from "@destack/environment";
import { Stage } from "@destack/update/release";
import { run } from "./command.ts";

/** The brand package's icon directory, resolved through its exports. */
const BRAND_ICONS = dirname(fileURLToPath(import.meta.resolve("@platform/brand/icon/icon.svg")));

/** Build every stage's macOS icons into the brand package. */
export async function buildIcons(environment: Environment<HostCapability>): Promise<void> {
    // refuse a host without the macOS image tools
    if (environment.host.os !== "darwin") {
        throw new Error("the macOS icons are built on macOS");
    }

    // build every stage's icons in a directory of their own
    const { files } = environment;
    const temporary = await files.makeTemporaryDirectory("destack-icon-");
    const tauri = join(CHECKOUT, "@destack/desktop/node_modules/@tauri-apps/cli/tauri.js");
    try {
        for (const { suffix } of Stage.names.map((name) => Stage.of(name))) {
            await buildMacIcon(suffix, tauri, join(temporary, `icon${suffix}`), environment);
        }
    } finally {
        await files.remove(temporary, { recursive: true, force: true });
    }
}

/** Pad a stage's raster icon as macOS applications are, and generate its `.icns` container. */
async function buildMacIcon(
    suffix: string,
    tauri: string,
    directory: string,
    environment: Environment<HostCapability>,
): Promise<void> {
    // apply the macOS padding to the rounded artwork
    const { files, process } = environment;
    const padded = join(BRAND_ICONS, `icon${suffix}-macos.png`);
    await files.mkdir(directory, { recursive: true });
    await run(
        "swift",
        [
            "-module-cache-path",
            join(directory, "swift"),
            join(CHECKOUT, "@platform/release/src/distribution/icon.swift"),
            join(BRAND_ICONS, `icon${suffix}-rounded.png`),
            padded,
        ],
        environment,
        CHECKOUT,
    );

    // generate the container beside the padded icon
    const words = ["run", tauri, "icon", padded, "--output", directory];
    await run(process.executable, words, environment, CHECKOUT);
    await files.copyFile(join(directory, "icon.icns"), join(BRAND_ICONS, `icon${suffix}.icns`));
}

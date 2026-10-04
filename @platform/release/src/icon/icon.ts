import { cp, mkdir, mkdtemp, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { run } from "../distribution/command.ts";

/** Repository containing the source artwork and native icon tools. */
const ROOT = fileURLToPath(new URL("../../../../", import.meta.url));

/** Build stable, nightly and development icons from their vector artwork. */
async function build(): Promise<void> {
    // prepare the output directory and isolate intermediate platform files
    const output = join(ROOT, "dist/icon");
    const temporary = await mkdtemp(join(tmpdir(), "destack-icon-"));
    const tauri = join(ROOT, "@destack/desktop/node_modules/@tauri-apps/cli/tauri.js");
    await mkdir(output, { recursive: true });

    // build every identity from its corresponding authored artwork
    try {
        for (const suffix of ["", "-nightly", "-dev"]) {
            await buildIdentity(suffix, tauri, temporary, output);
        }
    } finally {
        await rm(temporary, { recursive: true, force: true });
    }
}

/** Build the icons of one identity, named by its artwork suffix. */
async function buildIdentity(
    suffix: string,
    tauri: string,
    temporary: string,
    output: string,
): Promise<void> {
    // generate standard raster sizes
    const name = `destack${suffix}`;
    const source = join(ROOT, `@platform/brand/icon/icon${suffix}-rounded.svg`);
    const directory = join(temporary, name);
    await run(process.execPath, ["run", tauri, "icon", source, "--output", directory], ROOT);
    await cp(source, join(output, `${name}.svg`));
    await cp(join(directory, "icon.png"), join(output, `${name}.png`));

    // refresh the checked-in brand variants when requested
    if (process.argv.includes("--brand")) {
        await refreshBrand(suffix, tauri, source, join(directory, "brand"));
    }

    // build the native icon container on macOS
    if (process.platform === "darwin") {
        await buildMacIcon(name, tauri, temporary, directory, output);
    }
}

/** Rasterize the artwork at each brand size into the checked-in brand icons. */
async function refreshBrand(
    suffix: string,
    tauri: string,
    source: string,
    raster: string,
): Promise<void> {
    // rasterize the brand sizes
    const sizes = [180, 256, 512, 1024];
    const options = sizes.flatMap((size) => ["--png", String(size)]);
    await run(
        process.execPath,
        ["run", tauri, "icon", source, "--output", raster, ...options],
        ROOT,
    );

    // copy each size under its brand filename
    for (const size of sizes) {
        const filename =
            size === 1024 ? `icon${suffix}-rounded.png` : `icon${suffix}-rounded-${size}.png`;
        await cp(join(raster, `${size}x${size}.png`), join(ROOT, "@platform/brand/icon", filename));
    }
}

/** Pad the raster icon for macOS and generate its `.icns` container. */
async function buildMacIcon(
    name: string,
    tauri: string,
    temporary: string,
    directory: string,
    output: string,
): Promise<void> {
    // apply the macOS padding before generating the native icon container
    const padded = join(directory, "macos.png");
    await run(
        "swift",
        [
            "-module-cache-path",
            join(temporary, "swift"),
            join(ROOT, "@platform/release/src/icon/macos.swift"),
            join(directory, "icon.png"),
            padded,
        ],
        ROOT,
    );

    // generate the container and copy both files
    const macos = join(directory, "macos");
    await run(process.execPath, ["run", tauri, "icon", padded, "--output", macos], ROOT);
    await cp(padded, join(output, `${name}-macos.png`));
    await cp(join(macos, "icon.icns"), join(output, `${name}.icns`));
}

await build();

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
            // generate standard raster sizes and the Windows icon container
            const name = `destack${suffix}`;
            const source = join(ROOT, `platform/brand/icon/icon${suffix}.svg`);
            const directory = join(temporary, name);
            await run(
                process.execPath,
                ["run", tauri, "icon", source, "--output", directory],
                ROOT,
            );
            await cp(source, join(output, `${name}.svg`));
            await cp(join(directory, "icon.png"), join(output, `${name}.png`));
            await cp(join(directory, "icon.ico"), join(output, `${name}.ico`));

            // refresh the checked-in brand variants when requested
            if (suffix && process.argv.includes("--brand")) {
                const raster = join(directory, "brand");
                await run(
                    process.execPath,
                    [
                        "run",
                        tauri,
                        "icon",
                        source,
                        "--output",
                        raster,
                        "--png",
                        "180",
                        "--png",
                        "256",
                        "--png",
                        "512",
                        "--png",
                        "1024",
                    ],
                    ROOT,
                );
                for (const size of [180, 256, 512, 1024]) {
                    const filename =
                        size === 1024 ? `icon${suffix}.png` : `icon${suffix}-${size}.png`;
                    await cp(
                        join(raster, `${size}x${size}.png`),
                        join(ROOT, "platform/brand/icon", filename),
                    );
                }
            }

            // apply the macOS padding before generating the native icon container
            if (process.platform === "darwin") {
                const padded = join(directory, "macos.png");
                await run(
                    "swift",
                    [
                        "-module-cache-path",
                        join(temporary, "swift"),
                        join(ROOT, "platform/release/src/icon/macos.swift"),
                        join(directory, "icon.png"),
                        padded,
                    ],
                    ROOT,
                );
                const macos = join(directory, "macos");
                await run(
                    process.execPath,
                    ["run", tauri, "icon", padded, "--output", macos],
                    ROOT,
                );
                await cp(padded, join(output, `${name}-macos.png`));
                await cp(join(macos, "icon.icns"), join(output, `${name}.icns`));
            }
        }
    } finally {
        await rm(temporary, { recursive: true, force: true });
    }
}

await build();

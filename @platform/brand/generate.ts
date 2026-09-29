import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

const directory = fileURLToPath(new URL(".", import.meta.url));
// derive the rounded outline from each square master
for (const channel of ["", "-nightly", "-dev"]) {
    const name = join(directory, "icon", `icon${channel}`);
    const source = await readFile(`${name}.svg`, "utf8");
    const rectangle = '<rect width="32" height="32"/>';
    if (!source.includes(rectangle)) {
        throw new Error("missing square icon clip");
    }
    await writeFile(
        `${name}-rounded.svg`,
        source.replace(rectangle, '<rect width="32" height="32" rx="7"/>'),
    );

    // preserve existing rounded rasters; the release icon tool owns those exports
    for (const size of [180, 256, 512, 1024]) {
        const output = `${name}${size === 1024 ? "" : `-${size}`}.png`;
        execFileSync("rsvg-convert", [
            "-w",
            String(size),
            "-h",
            String(size),
            "-o",
            output,
            `${name}.svg`,
        ]);
    }
}

// render each social layout at its intended upload size
for (const name of ["x-header", "linkedin-company", "linkedin-profile"]) {
    const source = join(directory, "banner", `${name}.svg`);
    execFileSync("rsvg-convert", ["-o", source.replace(/\.svg$/, ".png"), source]);
}

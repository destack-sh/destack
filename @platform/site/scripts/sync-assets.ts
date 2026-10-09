import { cp, mkdir, rm } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** The site package directory. */
const siteDirectory = new URL("..", import.meta.url).pathname;
/** The public copy of the brand assets. */
const publicBrandDirectory = join(siteDirectory, "public", "brand");
/** The brand package directory, resolved through its exports. */
const brandDirectory = dirname(fileURLToPath(import.meta.resolve("@platform/brand/mark.svg")));

// replace the public brand assets
await rm(publicBrandDirectory, { recursive: true, force: true });
await mkdir(join(publicBrandDirectory, "banner"), { recursive: true });

// copy the mark, the icons, the favicons and the social preview
for (const asset of ["favicon", "icon", "mark.svg", "banner/og.png"]) {
    await cp(join(brandDirectory, asset), join(publicBrandDirectory, asset), { recursive: true });
}

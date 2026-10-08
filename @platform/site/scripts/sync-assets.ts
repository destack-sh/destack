import { cp, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";

/** The site package directory. */
const siteDirectory = new URL("..", import.meta.url).pathname;
/** The repository root. */
const repositoryDirectory = new URL("../../..", import.meta.url).pathname;
/** The public asset directory. */
const publicDirectory = join(siteDirectory, "public");
/** The public copy of the brand assets. */
const publicBrandDirectory = join(publicDirectory, "brand");
/** The brand package directory. */
const brandDirectory = join(repositoryDirectory, "@platform/brand");
/** The generated asset directory the bundle imports from. */
const generatedDirectory = join(siteDirectory, ".generated");

// copy the mark for the bundle
await mkdir(generatedDirectory, { recursive: true });
await cp(join(brandDirectory, "mark.svg"), join(generatedDirectory, "mark.svg"));

// replace the public brand assets
await mkdir(publicDirectory, { recursive: true });
await rm(publicBrandDirectory, { recursive: true, force: true });
await mkdir(publicBrandDirectory, { recursive: true });

for (const asset of ["favicon", "icon", "mark.svg"]) {
    await cp(join(brandDirectory, asset), join(publicBrandDirectory, asset), { recursive: true });
}

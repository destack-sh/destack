import { cp, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";

const siteDirectory = new URL("..", import.meta.url).pathname;
const repositoryDirectory = new URL("../../..", import.meta.url).pathname;
const publicDirectory = join(siteDirectory, "public");
const publicBrandDirectory = join(publicDirectory, "brand");
const brandDirectory = join(repositoryDirectory, "platform/brand");

await mkdir(publicDirectory, { recursive: true });
await rm(publicBrandDirectory, { recursive: true, force: true });
await mkdir(publicBrandDirectory, { recursive: true });

for (const asset of ["banner", "favicon", "icon"]) {
    await cp(join(brandDirectory, asset), join(publicBrandDirectory, asset), { recursive: true });
}

import { cp, mkdir } from "node:fs/promises";
import { join } from "node:path";

const siteDirectory = new URL("..", import.meta.url).pathname;
const repositoryDirectory = new URL("../../..", import.meta.url).pathname;
const outputDirectory = join(siteDirectory, ".output/public");

await mkdir(join(outputDirectory, "brand"), { recursive: true });

for (const asset of ["banner", "favicon", "icon"]) {
    await cp(join(repositoryDirectory, "platform/brand", asset), join(outputDirectory, "brand", asset), {
        recursive: true,
    });
}

await cp(join(repositoryDirectory, "app/cli/install/install.sh"), join(outputDirectory, "install"));
await cp(
    join(repositoryDirectory, "app/cli/install/install.ps1"),
    join(outputDirectory, "install.ps1"),
);

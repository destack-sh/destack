import { buildPackage } from "@destack/build";
import { readDependencies } from "@destack/build/local";
import { fileURLToPath } from "node:url";
import { mkdir, mkdtemp, readFile, writeFile, rename, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { PackageDefinition } from "@destack/package";
import configuration from "../../config.json" with { type: "json" };
import { ServiceConnectionDescription } from "@destack/service/inspect";
import { schema } from "@destack/schema";
import { DeclarationDescription } from "@destack/package/inspect";

/** Distribution-selected package supplying the desktop's initial view. */
const directory = fileURLToPath(
    new URL(`../../../../${configuration.desktop.package}`, import.meta.url),
);
/** Authored declarations validated before assembling the bundled application. */
const definition = PackageDefinition.parse(
    JSON.parse(await readFile(join(directory, "destack.json"), "utf8")),
);
/** Explicitly approved bundled view. */
const view = definition.views?.[configuration.desktop.view];
if (!view) {
    throw new Error("distribution view is not declared by the selected package");
}
/** Compiled browser package and its inspected declarations. */
await using build = await buildPackage({
    directory,
    dependencies: await readDependencies(directory),
    outputs: { app: { kind: "web", view: configuration.desktop.view, ssr: false, minify: true } },
});

/** Destination for the bundled package and launch description. */
const desktop = resolve(
    process.argv[2] ?? fileURLToPath(new URL("../../../../dist/desktop", import.meta.url)),
);
/** Digest identifying this exact bundled package. */
const manifest = new Bun.CryptoHasher("sha256")
    .update(JSON.stringify(build.manifest))
    .digest("hex");
/** Relative package directory recorded in the launch description. */
const directoryName = `package/${manifest}`;
/** Absolute destination used to publish the compiled package. */
const destination = join(desktop, directoryName);
await mkdir(join(desktop, "package"), { recursive: true });
try {
    const existing = await readFile(join(destination, "manifest.json"), "utf8");
    if (new Bun.CryptoHasher("sha256").update(existing).digest("hex") !== manifest) {
        throw new Error("existing desktop package manifest differs");
    }
} catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
        throw error;
    }
    // publish only complete builds and remove interrupted staging directories
    const staging = await mkdtemp(join(desktop, ".package-"));
    try {
        await build.write(join(staging, "build"));
        await rename(join(staging, "build"), destination);
    } finally {
        await rm(staging, { recursive: true, force: true });
    }
}

/** Inspected service declarations used by the selected browser output. */
const declarations = build.manifest.descriptions.service
    ? await build.reader.domain("service", schema.array(DeclarationDescription))
    : [];
/** Declaration indices referenced by the browser output. */
const selection = build.manifest.outputs["app-browser"].descriptions.service ?? [];
/** Connections to resolve against the configured distribution services. */
const connections = selection
    .map((index) => declarations[index])
    .filter((declaration) => declaration.kind === "service-connection")
    .map((declaration) => ServiceConnectionDescription.parse(declaration.description));
/** Temporary launch file published after its package and connections are complete. */
const launch = join(desktop, `${crypto.randomUUID()}.json`);
await writeFile(
    launch,
    JSON.stringify({
        title: configuration.desktop.title,
        profile: definition.id.slice("package-".length),
        source: {
            kind: "package",
            directory: directoryName,
            manifest,
            output: "app-browser",
            authorization: { packageId: definition.id, permissions: view.permissions ?? [] },
            connections: connections.map((connection) => {
                // resolve bundled dependencies against the distribution's explicit local services
                const targets = configuration.desktop.services.filter(
                    (target) =>
                        target.service.packageId === connection.service.packageId &&
                        target.service.name === connection.service.name,
                );
                if (targets.length !== 1) {
                    throw new Error(
                        `bundled connection requires one target: ${connection.packageId}/${connection.name}`,
                    );
                }

                return {
                    declaration: { packageId: connection.packageId, name: connection.name },
                    url: targets[0].url,
                };
            }),
        },
    }),
);
await rename(launch, join(desktop, "launch.json"));

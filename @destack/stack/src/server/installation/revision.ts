import { Catalog } from "@destack/locale";
import { describeSetting, SettingCatalog } from "@destack/setting";
import { declaration, installationRevision } from "../../object/index.ts";
import { BuildStore, type Registry } from "./build.ts";

/** Implement revisions with the views, settings and declarations their builds declare. */
export function implementInstallationRevision(builds: BuildStore, registry: Registry) {
    return installationRevision.implement({
        methods: {
            create: async (call, next) => {
                // reuse the revision of the same digest
                const { packageId, build, installationId, digest } = call.input;
                const [existing] = await call
                    .read(installationRevision)
                    .findMany({ where: { installationId, digest } });
                if (existing !== undefined) {
                    return existing;
                }

                // keep a release in the build store and read the build
                const reader =
                    build.kind === "release"
                        ? await BuildStore.open(builds, registry, packageId, build.manifest)
                        : await builds.packages.contents(build.manifest);

                // read the views the build's outputs mount, the settings it declares and its translations
                const views = Object.fromEntries(
                    Object.entries(reader.manifest.outputs).flatMap(([output, described]) =>
                        Object.entries(described.views).map(([name, view]) => [
                            name,
                            { ...view, output },
                        ]),
                    ),
                );
                const settings = (await SettingCatalog.read(reader)).settings.map(describeSetting);
                const catalogs = await Catalog.read(reader);
                const revision = await next(
                    call.with({ input: { ...call.input, views, settings, catalogs } }),
                );

                // keep each of the build's own declarations as an object readers query
                const declarations = await reader.declarations();
                await call.mutate(
                    call.caller,
                    declarations.map((declared) =>
                        declaration.call("create", {
                            revisionId: revision.id,
                            installationId: revision.installationId,
                            packageId,
                            kindPackageId: declared.package,
                            kind: declared.kind,
                            name: declared.name,
                            description: declared.description,
                        }),
                    ),
                );

                return revision;
            },
        },
    });
}

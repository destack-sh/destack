import type { PackageId } from "@destack/package";
import { describeSetting, SettingCatalog } from "@destack/setting/inspect";
import type { InstallationBuild } from "../../declare/installation.ts";
import { installationRevision } from "../../object/index.ts";
import type { OpenBuild } from "./release.ts";

/** Serve installation revisions with the views and settings their builds declare, read from the cell's store of builds. */
export function serveRevisions(openBuild: OpenBuild) {
    return installationRevision.handle({
        create: async (call, next) => {
            // read the views the build's outputs mount and the settings it declares
            const { packageId, build } = call.input as {
                readonly packageId: PackageId;
                readonly build: InstallationBuild;
            };
            const reader = await openBuild(packageId, build);
            const views = Object.fromEntries(
                Object.entries(reader.manifest.outputs).flatMap(([output, described]) =>
                    Object.entries(described.views).map(([name, view]) => [
                        name,
                        { ...view, output },
                    ]),
                ),
            );
            const settings = (await SettingCatalog.read(reader)).settings.map(describeSetting);

            return next(call.with({ input: { ...call.input, views, settings } }));
        },
    });
}

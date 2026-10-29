import type { Digest } from "@destack/schema";
import { installableReleases } from "../../declare/index.ts";
import type { PackageId } from "@destack/package";
import { space } from "@destack/account/object";
import { Build, installationRevision } from "../../object/index.ts";
import { type BuildStore, type Registry } from "./build.ts";

/** Implement revisions with the installable packages their builds depend on. */
export function implementInstallationRevision(builds: BuildStore, registry: Registry) {
    return installationRevision.implement({
        methods: {
            create: async (call, next) => {
                // reuse the revision of the same digest
                const { buildId, installationId, digest } = call.input;
                const [existing] = await call
                    .read(installationRevision)
                    .findMany({ where: { installationId, digest } });
                if (existing !== undefined) {
                    return existing;
                }

                // read the space's build from the build store
                const built = await Build.require(call, space.identifier(call.scope), buildId);
                const reader = await builds.contents(built.manifest);

                // read the installable packages it depends on, at the releases it compiled against, without storing them
                const read = (dependency: PackageId, manifest: Digest) =>
                    builds.pull(registry, dependency, manifest);
                const dependencies = await installableReleases(reader, read);

                return next(
                    call.with({
                        input: { ...call.input, dependencies },
                    }),
                );
            },
        },
    });
}

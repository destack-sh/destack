import { InstallationRevision, installationRevision } from "../../object/index.ts";

/** Implement revisions, reusing the revision of the same digest. */
export function implementInstallationRevision() {
    return installationRevision.implement({
        methods: {
            create: async (call, next) => {
                // identify the revision by its installation and digest, reusing it once recorded
                const { installationId, digest } = call.input;
                const id = await InstallationRevision.id(
                    installationId,
                    digest,
                    call.environment.crypto,
                );
                const [existing] = await call
                    .read(installationRevision)
                    .findMany({ where: { id } });

                return existing ?? (await next(call.identify(id)));
            },
        },
    });
}

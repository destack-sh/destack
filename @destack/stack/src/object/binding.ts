import { type Snapshot } from "@destack/db";
import { installation } from "@destack/object";
import { PackageId } from "@destack/package";
import { type Identifier, present, schema } from "@destack/schema";
import { Deployment, deployment } from "./deployment.ts";
import { type InstallationRevision, installationRevision, rollout } from "./rollout.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../error/index.ts";

/** One declaration of a revision bound to a target at the version it runs with. */
export const Binding = Object.assign(
    schema.object({
        /** The package declaring the need. */
        packageId: PackageId,
        /** The declaration name within the package. */
        name: schema.string().min(1),
        /** The bound object's id here, or the installation's name in the target space. */
        target: schema.string().min(1),
        /** The other space of the target installation, null for this space. */
        targetSpaceId: schema.identifier("space").nullable(),
        /** The target's version or generation the revision runs with, null in another space. */
        version: schema.number().int().positive().nullable(),
    }),
    {
        /** List the bindings of an installation's targeted revision, none before it targets one. */
        async targeted(
            reader: Pick<Snapshot, "read">,
            scope: Identifier<"space">,
            installationId: Identifier<"installation">,
        ): Promise<Binding[]> {
            const [row] = await reader
                .read(rollout.table)
                .findMany({ where: { scope, installationId } });
            const revision =
                row?.revisionId === null || row === undefined
                    ? undefined
                    : await reader
                          .read(installationRevision.table)
                          .findFirst({ where: { id: row.revisionId } });

            return revision === undefined ? [] : revision.bindings;
        },

        /** List the installations whose targeted revision binds a target of a space. */
        async binders(
            reader: Pick<Snapshot, "read">,
            scope: Identifier<"space">,
            id: string,
        ): Promise<Identifier<"installation">[]> {
            // read the space's targeted revisions of live installations, and take the ones binding the target here
            const rollouts = await liveRollouts(reader, scope);
            const ids = rollouts.flatMap((row) =>
                row.revisionId === null ? [] : [row.revisionId],
            );
            const revisions = await reader
                .read(installationRevision.table)
                .findMany({ where: { id: { in: ids } } });

            return revisions
                .filter((revision) =>
                    revision.bindings.some(
                        (bound) => bound.targetSpaceId === null && bound.target === id,
                    ),
                )
                .map((revision) => revision.installationId);
        },

        /** Refuse destroying a target's version while a live deployment's revision runs with it. */
        async requireReleased(
            reader: Pick<Snapshot, "read">,
            target: {
                readonly type: string;
                readonly scope: string;
                readonly id: string;
                readonly version: number;
            },
        ): Promise<void> {
            // leave a target outside spaces, which no revision binds
            const parsed = schema.identifier("space").safeParse(target.scope);
            if (!parsed.success) {
                return;
            }

            // refuse a version a live deployment's revision runs with
            const running = await liveRevision(reader, parsed.data, target.id, target.version);
            if (running !== undefined) {
                const name = await installationName(reader, running);
                throw StackError.of(
                    "VERSION_BOUND",
                    `${target.type} version ${target.version} is bound by ${name}`,
                );
            }
        },

        /** Refuse removing a target while a current revision binds it. */
        async requireUnbound(
            reader: Pick<Snapshot, "read">,
            target: { readonly scope: string; readonly id: string },
        ): Promise<void> {
            // leave a target outside spaces, which no installation binds
            const parsed = schema.identifier("space").safeParse(target.scope);
            if (!parsed.success) {
                return;
            }

            // refuse a target bound by a revision a rollout targets or a live deployment runs
            const bound = await liveRevision(reader, parsed.data, target.id, undefined);
            if (bound !== undefined) {
                const name = await installationName(reader, bound);
                throw StackError.of("BOUND", `${target.id} is in use: bound by ${name}`);
            }
        },
    },
);
/** One declaration of a revision bound to a target at a version. */
export type Binding = schema.Infer<typeof Binding>;

/** Read the name of the installation a revision belongs to. */
async function installationName(
    reader: Pick<Snapshot, "read">,
    revision: Pick<InstallationRevision, "installationId">,
): Promise<string> {
    const installed = await reader
        .read(installation.table)
        .findFirst({ where: { id: revision.installationId } });

    return present(installed?.name, `the name of installation ${revision.installationId}`);
}

/** Read a current revision of a space binding a target, at a version when given. */
async function liveRevision(
    reader: Pick<Snapshot, "read">,
    scope: Identifier<"space">,
    id: string,
    version: number | undefined,
): Promise<InstallationRevision | undefined> {
    // read the revisions live installations' rollouts target and the live deployments run
    const rollouts = await liveRollouts(reader, scope);
    const live = await reader
        .read(deployment.table)
        .findMany({ where: { ...Deployment.live(), scope } });
    const current = [
        ...rollouts.flatMap((row) =>
            version === undefined && row.revisionId !== null ? [row.revisionId] : [],
        ),
        ...live.map((row) => row.revisionId),
    ];
    const revisions = await reader
        .read(installationRevision.table)
        .findMany({ where: { id: { in: [...new Set(current)] } } });

    // take one binding the target at the version
    const binding = revisions.find((revision) =>
        revision.bindings.some(
            (bound) =>
                bound.targetSpaceId === null &&
                bound.target === id &&
                (version === undefined || bound.version === version),
        ),
    );

    return binding;
}

/** Read the rollouts of a space's installations that are not being deleted. */
async function liveRollouts(reader: Pick<Snapshot, "read">, scope: Identifier<"space">) {
    // read the space's rollouts and its installations being deleted
    const rollouts = await reader.read(rollout.table).findMany({ where: { scope } });
    const deleted = await reader
        .read(installation.table)
        .findMany({ where: { scope, deletedAt: { isNotNull: true } } });
    const gone = new Set(deleted.map((row) => row.id));

    return rollouts.filter((row) => !gone.has(row.installationId));
}

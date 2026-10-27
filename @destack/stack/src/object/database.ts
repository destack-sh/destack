import { DatabaseKind, type Select, type Snapshot } from "@destack/db";
import { MAIN_DATABASE } from "../declare/index.ts";
import { defineObject, OBJECT_DECLARATION, Observation } from "@destack/object";
import { space } from "@destack/account/object";
import { sudo } from "@destack/account/authentication";
import { Digest, type DigestCrypto, type Identifier, present } from "@destack/schema";
import { RESOURCE_DECLARATION } from "@destack/resource";
import { binding } from "./binding.ts";
import { Build, build } from "./build.ts";
import {
    type InstallationRevision,
    installationRevision,
    Rollout,
    rollout,
    type NamedInstallation,
} from "./rollout.ts";
import type { graph } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";

/** The provider that stores a space's main database in the space's own storage. */
export const SPACE_PROVIDER = "space";

/** A space's database resource. */
export const database = defineObject({
    name: "database",
    plural: "databases",
    scope: space,
    provisioned: { kind: DatabaseKind },
    fields: {},
    administration: ["read", "manage"],
    elevated: { manage: sudo },
});

/** A persisted database resource. */
export type Database = Select<typeof database.table>;

/** The databases of spaces. */
export const Database = {
    /** Read a space's main database, which the space has from its birth on. */
    async main(snapshot: Pick<Snapshot, "read">, spaceId: Identifier<"space">): Promise<Database> {
        return present(
            await Database.mainMaybe(snapshot, spaceId),
            `the main database of ${spaceId}`,
        );
    },

    /** Read a space's main database, absent where its row has not arrived yet. */
    async mainMaybe(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
    ): Promise<Database | undefined> {
        return snapshot
            .read(database.table)
            .findFirst({ where: { scope: spaceId, name: MAIN_DATABASE } });
    },

    /** Read the revision a space serves each installation at, with its build and declarations. */
    async installed(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        open: (manifest: Digest) => Promise<Pick<BuildReader, "declarations">>,
        crypto: DigestCrypto,
    ): Promise<
        {
            readonly rollout: Rollout & { readonly installation: NamedInstallation };
            readonly revisionId: InstallationRevision["id"];
            readonly manifest: Digest;
            readonly declarations: graph.Declaration[];
        }[]
    > {
        // read no installations of a space before its birth gives it main
        const [main] = await snapshot
            .read(database.table)
            .findMany({ where: { scope: spaceId, name: MAIN_DATABASE } });
        if (main === undefined) {
            return [];
        }

        // read the rollouts with their installations, their bindings, and their revisions' builds
        const installations = await Rollout.read(
            snapshot,
            await snapshot.read(rollout.table).findMany({ where: { scope: spaceId } }),
        );
        const bindings = await snapshot.read(binding.table).findMany({ where: { scope: spaceId } });
        const revisionIds = installations.flatMap((entry) =>
            [entry.revisionId, entry.appliedRevisionId].filter((id) => id !== null),
        );
        const revisions = await snapshot
            .read(installationRevision.table)
            .findMany({ where: { scope: spaceId, id: { in: revisionIds } } });
        const builds = await snapshot.read(build.table).findMany({
            where: { scope: spaceId, id: { in: revisions.map((entry) => entry.buildId) } },
        });

        // read each revision's build and its declarations, once
        const declared = new Map(
            await Promise.all(
                revisions.map(async (revision) => {
                    // open the revision's build by its manifest
                    const built = present(
                        builds.find((each) => each.id === revision.buildId),
                        `the build of revision ${revision.id}`,
                    );
                    const manifest = Build.manifest(built);
                    const reader = await open(manifest);

                    return [
                        revision.id,
                        { manifest, declarations: await reader.declarations() },
                    ] as const;
                }),
            ),
        );

        // serve each installation at its revision once its targets are bound and main applied its tables, else at the revision it deployed last
        const applied = new Set(main.appliedStates);
        const served = await Promise.all(
            installations.map(async (entry) => {
                // serve a bound revision declaring no object types at once, and one declaring some once main applied them
                const latest =
                    entry.revisionId === null
                        ? undefined
                        : declared.get(entry.revisionId)?.declarations;
                const tables = bindings.find(
                    (each) =>
                        each.installationId === entry.id &&
                        each.name === MAIN_DATABASE &&
                        each.target === main.id,
                );
                const isBound =
                    Observation.isCurrent(entry, "Bound") &&
                    entry.conditions["Bound"]?.status === "true";
                const isApplied =
                    latest !== undefined &&
                    isBound &&
                    (!latest.some((each) => each.kind === OBJECT_DECLARATION) ||
                        (tables !== undefined &&
                            applied.has(await Digest.json(tables.state, crypto))));
                const revisionId = isApplied ? entry.revisionId : entry.appliedRevisionId;

                return revisionId === null ? [] : [{ rollout: entry, revisionId }];
            }),
        );

        // select each one whose own resources are all bound, since servers open them at start and read secrets on use
        const bound = new Set(
            bindings.map((entry) => `${entry.installationId}:${entry.packageId}:${entry.name}`),
        );

        return served.flat().flatMap(({ rollout: entry, revisionId }) => {
            const { manifest, declarations } = present(
                declared.get(revisionId),
                `revision ${revisionId}`,
            );
            const hasBindings = declarations
                .filter((each) => each.kind === RESOURCE_DECLARATION)
                .every((each) =>
                    bound.has(`${entry.id}:${entry.installation.packageId}:${each.name}`),
                );

            return hasBindings ? [{ rollout: entry, revisionId, manifest, declarations }] : [];
        });
    },
};

import { schema } from "@destack/schema";
import {
    AccessContext,
    accessRelationship,
    Caller,
    Permission,
    principal,
    Relationship,
} from "@destack/access";
import { account, residency, space, user } from "@destack/account/object";
import { Snapshot, Table } from "@destack/db";
import { grant, role, type PackageDescription } from "@destack/object";
import { nth } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { ids, insertRollout, openSpaceDatabase } from "../test/index.ts";
import { Rollout, rollout } from "@destack/stack/object";
import { InstallationAccess } from "./access.ts";

/** The description of a notes release requesting reading the space and its own notes. */
const DESCRIPTION: PackageDescription = {
    api: { name: "notes", procedures: [] },
    objects: {},
    mounts: {},
    views: [],
    permissions: {
        space: {
            space: [
                space.permission("read"),
                Permission.parse({ packageId: ids.package, type: "note", name: "read" }),
            ],
            account: [account.permission("read")],
            universe: [user.permission("recipient")],
        },
    },
    capabilities: {},
    errors: [],
};

/** The person installing notes. */
const INSTALLER = principal.user.reference(
    Scope.universe.id,
    "user-01996ab0-0000-7000-8000-0000000000b1",
);

/** Open a migrated space database with the space's row and its notes rollout. */
async function installNotes(): ReturnType<typeof openSpaceDatabase> {
    // open the space's database
    const opened = await openSpaceDatabase();

    // record its notes rollout
    await insertRollout(
        opened.database,
        {
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            selection: { kind: "release", version: "2026.9.0" },
            installer: INSTALLER,
        },
        opened.now,
    );

    return opened;
}

test("bind an installation's requests on other packages' types on its consenter's authority, and drop them once it goes", async () => {
    // install notes in the space as its owner
    const { database, now } = await installNotes();
    const installer = INSTALLER;
    const snapshot = Snapshot.live(database);
    const rows = await snapshot
        .read(rollout.table)
        .findMany({ where: { installationId: ids.notes } });
    const target = nth(await Rollout.read(snapshot, rows), 0);

    // build its access as the space, whose own code installs it, then once it goes
    const subject = principal.space.reference(Scope.universe.id, ids.space);
    const installation = principal.installation.reference(ids.space, ids.notes);
    const installed = await InstallationAccess.changes(snapshot, target, DESCRIPTION, subject, now);
    const removed = await InstallationAccess.changes(snapshot, target, undefined, subject, now);

    // run the consented role for the installer
    const binding = (permissions: readonly Permission[]) =>
        role.call("declare", {
            ...role.in(ids.space),
            name: `installation/${ids.notes}`,
            description: "What notes requests in this space",
            permissions: [...permissions],
            subjects: [installation],
            manager: { installationId: ids.notes, packageId: ids.package, name: "permissions" },
        });
    const delegator = AccessContext.of(Caller.of(subject), now);
    const lent = grant.call("create", {
        ...grant.in(ids.space),
        delegate: installation,
        permissions: [
            { ...account.permission("read"), scope: ids.account },
            user.permission("recipient"),
        ],
    });
    const consenter = AccessContext.of(Caller.acting(installer, subject), now);
    expect({ installed, removed }).toEqual({
        installed: [
            { caller: consenter, calls: [binding([space.permission("read")])] },
            // grant the account-scoped request and the universe-scoped one in every scope as the space
            { caller: delegator, calls: [lent] },
        ],
        removed: [{ caller: consenter, calls: [binding([])] }],
    });
});

test("relate a residency home's declared installation while it asks the universe, unrelate it once it stops, leave a settled relationship, and relate it again after an upgrade", async () => {
    // declare notes in the space, which homes a residency
    const { database, now } = await installNotes();
    const lab = schema
        .identifier("residency")
        .parse("residency-01996ab0-0000-7000-8000-00000000000e");
    await database.transaction(
        async (transaction) =>
            await transaction.insert(residency.table).values(
                Table.complete(residency.table, {
                    id: lab,
                    scope: Scope.universe.id,
                    code: "lab",
                    name: "Lab",
                    relay: "https://relay-lab.machine.test",
                    space: ids.space,
                    createdAt: now,
                    updatedAt: now,
                }),
            ),
    );
    const snapshot = Snapshot.live(database);
    const rows = await snapshot
        .read(rollout.table)
        .findMany({ where: { installationId: ids.notes } });
    const read = nth(await Rollout.read(snapshot, rows), 0);
    const stack = schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-00000000000d");
    const managed = { ...read.installation, managerInstallationId: stack };
    const target = { ...read, installation: managed };

    // relate an unrelated installation asking the universe, and leave one asking nothing
    const set = { universe: [user.permission("recipient")] };
    const at = residency.target({ scope: Scope.universe.id, id: lab });
    const unrelated = {
        asking: await InstallationAccess.relate(snapshot, target, target, set, "2026.9.0"),
        silent: await InstallationAccess.relate(snapshot, target, target, {}, "2026.9.0"),
    };

    // relate it at the residency, then leave it while it asks, and unrelate it once it stops
    const related = Relationship.encode({
        id: "relationship-01996ab0-0000-7000-8000-00000000000f",
        object: residency.reference(Scope.universe.id, lab),
        relation: "installation",
        subject: principal.installation.reference(ids.space, ids.notes),
        createdAt: now,
        expiresAt: null,
    });
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(accessRelationship)
                .values(Table.complete(accessRelationship, related)),
    );
    const settled = {
        asking: await InstallationAccess.relate(snapshot, target, target, set, "2026.9.0"),
        silent: await InstallationAccess.relate(snapshot, target, target, {}, "2026.9.0"),
    };

    // relate it again once a deploy applies another revision
    const applied = schema
        .identifier("installation-revision")
        .parse("installation-revision-01996ab0-0000-7000-8000-000000000010");
    const upgraded = await InstallationAccess.relate(
        snapshot,
        target,
        { ...target, appliedRevisionId: applied },
        set,
        "2026.9.0",
    );
    const touch = residency.call("touch", {
        ...at,
        installation: ids.notes,
        name: "notes",
        packageId: ids.package,
        release: "2026.9.0",
    });
    expect({ unrelated, settled, upgraded }).toEqual({
        unrelated: { asking: [touch], silent: [] },
        settled: {
            asking: [],
            silent: [residency.call("revoke", { ...at, installation: ids.notes, name: "notes" })],
        },
        upgraded: [touch],
    });
});

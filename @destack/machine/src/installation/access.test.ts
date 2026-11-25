import { testIdentity } from "@destack/object/test";
import { nth } from "@destack/schema";
import { AccessContext, Caller, Permission, principal } from "@destack/access";
import { account, space, user } from "@destack/account/object";
import { grant, role, type PackageDescription } from "@destack/object";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { ids, insertInstallation, NOTES_RELEASE, openSpaceDatabase } from "../test/index.ts";
import { installation } from "@destack/stack/object";
import { InstallationAccess } from "./access.ts";

/** The description of a notes release requesting reading the space and its own notes. */
const DESCRIPTION: PackageDescription = {
    ...testIdentity,
    objects: {},
    protocols: [],
    mounts: {},
    views: [],
    follows: [],
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
    await insertInstallation(
        opened.database,
        {
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            version: NOTES_RELEASE,
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
    const snapshot = database.snapshot();
    const rows = await snapshot.read(installation.table).findMany({ where: { id: ids.notes } });
    const target = nth(rows, 0);

    // build its access as the space, whose own code installs it, then once it goes
    const subject = principal.space.reference(Scope.universe.id, ids.space);
    const runner = principal.installation.reference(ids.space, ids.notes);
    const installed = await InstallationAccess.changes(snapshot, target, DESCRIPTION, subject, now);
    const removed = await InstallationAccess.changes(snapshot, target, undefined, subject, now);

    // run the consented role for the installer
    const binding = (permissions: readonly Permission[]) =>
        role.call("declare", {
            ...role.in(ids.space),
            name: `installation/${ids.notes}`,
            description: "What notes requests in this space",
            permissions: [...permissions],
            subjects: [runner],
            manager: { installationId: ids.notes, packageId: ids.package, name: "permissions" },
        });
    const delegator = AccessContext.of(Caller.of(subject), now);
    const lent = grant.call("create", {
        ...grant.in(ids.space),
        delegate: runner,
        permissions: [
            { ...account.permission("read"), scope: ids.account },
            user.permission("recipient"),
        ],
    });
    const consenter = AccessContext.of(Caller.onBehalfOf(installer, subject), now);
    expect({ installed, removed }).toEqual({
        installed: [
            { caller: consenter, calls: [binding([space.permission("read")])] },
            // grant the account-scoped request and the universe-scoped one in every scope as the space
            { caller: delegator, calls: [lent] },
        ],
        removed: [{ caller: consenter, calls: [binding([])] }],
    });
});

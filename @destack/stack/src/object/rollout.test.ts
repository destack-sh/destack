import { AccessContext, Caller, Permission, principal } from "@destack/access";
import { account, residency, space, user } from "@destack/account/object";
import { Snapshot, Table } from "@destack/db";
import { grant, Installation, role, type PackageDescription } from "@destack/object";
import { nth } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import stackPackage from "../package.ts";
import { ids, insertRollout, openSpaceDatabase } from "../test/index.ts";
import { Rollout, rollout } from "./index.ts";

/** The description of a notes release requesting reading the space and its own notes. */
const DESCRIPTION: PackageDescription = {
    api: { name: "notes", procedures: [] },
    objects: {},
    adapters: {},
    views: [],
    permissions: {
        installation: {
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
    const { database, crypto, now } = await installNotes();
    const installer = INSTALLER;
    const snapshot = Snapshot.live(database);
    const rows = await snapshot.read(rollout.table).findMany({ where: { id: ids.notes } });
    const target = nth(await Rollout.read(snapshot, rows), 0);

    // build its access as the space's base stack installation, then once it goes
    const scope = principal.space.reference(Scope.universe.id, ids.space);
    const stack = await Installation.base(scope, stackPackage, crypto);
    const subject = principal.installation.reference(ids.space, stack.installation);
    const installation = principal.installation.reference(ids.space, ids.notes);
    const actor = { subject, now };
    const installed = await Rollout.access(snapshot, target, DESCRIPTION, actor);
    const removed = await Rollout.access(snapshot, target, undefined, actor);

    // run the consented role for the installer
    const binding = (permissions: readonly Permission[]) =>
        role.call("upsertBinding", {
            ...role.in(ids.space),
            name: `installation/${ids.notes}`,
            description: "What notes requests in this space",
            permissions: [...permissions],
            subject: installation,
            manager: { installationId: ids.notes, packageId: ids.package, name: "permissions" },
        });
    const delegator = AccessContext.of(Caller.acting(scope, subject), now);
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
        removed: [
            { caller: consenter, calls: [binding([])] },
        ],
    });
});

test("relate a residency home's declared installation for its universe-scoped requests, and unrelate it once it goes", async () => {
    // declare notes in the space, which homes a residency
    const { database, now } = await installNotes();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(residency.table).values(
                Table.complete(residency.table, {
                    id: "lab",
                    scope: Scope.universe.id,
                    name: "Lab",
                    space: ids.space,
                    createdAt: now,
                    updatedAt: now,
                }),
            ),
    );
    const snapshot = Snapshot.live(database);
    const rows = await snapshot.read(rollout.table).findMany({ where: { id: ids.notes } });
    const read = nth(await Rollout.read(snapshot, rows), 0);
    const target = { ...read, installation: { ...read.installation, origin: "declared" as const } };

    // relate it while it asks for the universe's permissions, and unrelate it once it goes
    const set = { universe: [user.permission("recipient")] };
    const at = residency.target({ scope: Scope.universe.id, id: "lab" });
    expect({
        installed: await Rollout.relate(snapshot, target, set),
        removed: await Rollout.relate(snapshot, target, {}),
    }).toEqual({
        installed: [
            residency.call("touch", {
                ...at,
                installation: ids.notes,
                name: "notes",
                packageId: ids.package,
                release: "2026.9.0",
            }),
        ],
        removed: [residency.call("revoke", { ...at, installation: ids.notes, name: "notes" })],
    });
});

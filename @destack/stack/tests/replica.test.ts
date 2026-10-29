import { FOLLOWER_REVISION } from "@destack/sync/test";
import accountPackage from "@destack/account/package";
import { CHAIN_PUBLICATION, principal } from "@destack/access";
import { user } from "@destack/account/object";
import { TABLE } from "@destack/db";
import { ObjectSink } from "@destack/object/server";
import { Identifier, schema } from "@destack/schema";
import { type ObjectTypeReference, type Page, RequestId, type Subject } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import stackPackage from "../src/package.ts";
import { ids, TestSpace } from "../src/test/index.ts";

test("relay the access deciding a space's objects to each of its installations alone", async () => {
    // serve the space below its account with an installation, next to another space of the account
    const { snapshot, notes, other } = await serveInstallation();

    // relay the space's row, its base roles and bindings, and the permissions on the build and space
    const base = ["object", "setting", "stack"].map((name) => `base:@destack/${name}`);
    expect(described(await snapshot(notes, ids.space))).toEqual([
        ...base.map((name) => ["destack__access__relationship", `${name} space`]),
        ...["cancel", "create", "push", "read", "record"].map((name) => [
            "destack__access__role_permission",
            `base:@destack/stack build.${name}`,
        ]),
        // include every unreserved permission of the space type the stack serves
        ...[
            "analyze",
            "comment",
            "control",
            "create",
            "delete",
            "list",
            "manage",
            "member",
            "observe",
            "read",
            "rotate",
            "unmask",
            "update",
            "write",
        ].map((name) => ["destack__access__role_permission", `base:@destack/stack space.${name}`]),
        ...base.map((name) => ["destack__access__role", name]),
        ["destack__sync__scope", ids.space],
    ]);
    expect(located(await snapshot(notes, ids.account))).toEqual([
        ["destack__access__relationship", ids.account],
        ["destack__sync__replica_position", ids.account],
        ["destack__sync__scope", ids.account],
    ]);

    // refuse another space's installation and a scope outside the space's chain
    expect(await refusal(snapshot(other, ids.space))).toEqual([
        "FORBIDDEN",
        "FOLLOWER_OUTSIDE_TARGET",
        `only ${ids.space} and its contents can follow its replicas`,
    ]);
    expect(await refusal(snapshot(notes, "account-elsewhere"))).toEqual([
        "NOT_FOUND",
        "SCOPE_NOT_CONTAINED",
        `account-elsewhere does not contain ${ids.space}`,
    ]);
});

test("relay the user enclosing a space to the space's installation, concealing its guarded fields", async () => {
    // serve the space below an account of its owner's user scope, the owner's user row replicated
    const { fixture, snapshot, notes } = await serveInstallation();
    const owner = fixture.owner.id;

    // relay her user row to the installation inside her scope
    const changes = await snapshot(notes, owner, user.typeReference);
    expect(
        changes.flatMap((change) =>
            change.table === user.table[TABLE].sqlName
                ? [[change.row["id"], change.concealed]]
                : [],
        ),
    ).toEqual([[owner, ["email", "emailVerified", "login", "timeZone", "suspendedAt"]]]);
});

/** Serve a space with one installation below its account, next to another space of the account. */
async function serveInstallation() {
    // install notes in the space as its owner, and create the account's other space at the universe
    const fixture = await TestSpace.open();
    const served = await fixture.serve();
    const owner = served.as(fixture.owner.subject).client(stackPackage);
    const installed = await owner.rollout.create({
        spaceId: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        kind: "package",
        selection: { kind: "release", version: "2026.9.0" },
        name: "notes",
    });
    const otherSpace = Identifier.create("space", fixture.environment);
    await fixture.owner.client(accountPackage).space.create({
        accountId: ids.account,
        id: otherSpace,
        requestId: RequestId.create(fixture.machine.environment),
        name: "other",
    });
    const notes = principal.installation.reference(ids.space, installed.id);
    const other = principal.installation.reference(
        otherSpace,
        Identifier.create("installation", fixture.environment),
    );

    // read the snapshot of one scope's chain replica, with a type it hands down, as an installation with each row's scope
    const snapshot = async (as: Subject, scope: string, type?: ObjectTypeReference) => {
        const pages = await served
            .as(as)
            .client(stackPackage)
            .store.stream({
                name: ObjectSink.chainName(ids.space),
                scope,
                target: ids.space,
                publication: CHAIN_PUBLICATION,
                parameters: {
                    scopes: [ids.space],
                    local: [],
                    replicated: [{ scope, types: type === undefined ? [] : [{ ...type }] }],
                    via: [],
                    between: [],
                },
                revision: FOLLOWER_REVISION,
            });
        const changes: Page["changes"][number][] = [];
        for await (const page of pages) {
            changes.push(...page.changes);
            if (page.complete) {
                break;
            }
        }

        return changes;
    };

    return { fixture, snapshot, notes, other };
}

/** Describe each relayed access row by its table and its role, permission or binding. */
function described(changes: Page["changes"]): [string, string][] {
    // name each role by its identifier
    const roles = new Map(
        changes
            .filter((change) => change.table === "destack__access__role")
            .map((change) => [change.row["id"], schema.string().parse(change.row["name"])]),
    );
    const roleOf = (row: Page["changes"][number]["row"]) => roles.get(row["roleId"]) ?? "";

    // label each row
    return changes
        .map((change): [string, string] => {
            const { row } = change;
            const label =
                change.table === "destack__access__role"
                    ? (roles.get(row["id"]) ?? "")
                    : change.table === "destack__access__role_permission"
                      ? `${roleOf(row)} ${schema.string().parse(row["type"])}.${schema.string().parse(row["name"])}`
                      : change.table === "destack__access__relationship"
                        ? `${roleOf(row)} ${schema.string().parse(row["objectType"])}`
                        : schema.string().parse(row["scope"]);

            return [change.table, label];
        })
        .toSorted((left, right) => left.join().localeCompare(right.join()));
}

/** List each change's table and scope in order. */
function located(changes: Page["changes"]): [string, string][] {
    return changes
        .map((change): [string, string] => [
            change.table,
            schema.string().parse(change.row["scope"]),
        ])
        .toSorted((left, right) => left.join().localeCompare(right.join()));
}

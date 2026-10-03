import {
    defineDatabase,
    defineTable,
    index,
    integer,
    TABLE,
    text,
    sql,
    type Tree,
} from "@destack/db";
import {
    none,
    Policy,
    principal,
    anyone,
    relation,
    union,
    permission,
    through,
    intersection,
    exclusion,
    resource,
    context,
} from "../index.ts";
import { accessTables, type TableMapping } from "../index.ts";
import { PackageId } from "@destack/package";

/** Declare objects under a fixed test package. */
export function testModule(number: number) {
    const id = PackageId.parse(`package-01996ab0-0000-7000-8000-00000000000${number}`);

    return {
        package: { id, name: `@example/package-${number}`, version: "2026.9.0" },
    };
}
/** The first test package. */
export const module1 = testModule(1);
/** The second test package. */
export const module2 = testModule(2);
/** The third test package. */
export const module3 = testModule(3);

/** The fourth test package, declaring the scope and group types the tests use. */
export const module4 = testModule(4);

/** The authentication sensitive test permissions ask for: several factors within fifteen minutes. */
const RECENT = { assurance: 2, maxAge: 15 * 60 * 1000 };

/** The permissions the test scope and group types grant through roles. */
const OBJECT_PERMISSIONS = { read: none(), update: none(), delete: none(), share: none() };

/** Accounts: the root scopes of spaces, with users as members. */
export const account = new Policy(module4.package, {
    name: "account",
    relations: {
        member: { subjects: [principal.user] },
        root: { subjects: [principal.user], grantedBy: null },
    },
    permissions: { ...OBJECT_PERMISSIONS, own: relation("root") },
    grantedBy: "share",
    reserved: ["own"],
    elevated: { delete: RECENT, own: RECENT },
    scope: true,
});

/** A space within an account. */
export const space = new Policy(module4.package, {
    name: "space",
    permissions: OBJECT_PERMISSIONS,
    grantedBy: "share",
    elevated: { delete: RECENT },
    scope: true,
});

/** A set of users, installations, nested groups and account members. */
export const group = new Policy(module4.package, {
    name: "group",
    relations: {
        member: {
            subjects: [
                principal.user,
                principal.installation,
                "group#member",
                account.members("member"),
            ],
        },
    },
    permissions: OBJECT_PERMISSIONS,
    grantedBy: "share",
});

/** Notes and nested nodes with direct and inherited sharing. */
export const node = new Policy(module1.package, {
    name: "node",
    attributes: {},
    relations: {
        owner: { subjects: [principal.user], grantedBy: null },
        parent: { subjects: ["node"], grantedBy: null },
        viewer: {
            subjects: [
                principal.user,
                principal.user.all(),
                group.members("member"),
                "team#member",
                anyone.all(),
            ],
        },
        editor: {
            subjects: [
                principal.user,
                group.members("member"),
                principal.installation,
                anyone.all(),
            ],
        },
        "subtree-editor": { subjects: [principal.user, anyone.all()] },
        auditor: { subjects: [principal.user], concealed: true },
    },
    permissions: {
        share: relation("owner"),
        "edit-descendant": union(relation("owner"), relation("subtree-editor")),
        edit: union(
            relation("owner"),
            relation("editor"),
            relation("subtree-editor"),
            through("parent", "edit-descendant", true),
        ),
        read: union(permission("edit"), relation("viewer"), relation("auditor")),
    },
    grantedBy: "share",
});

/** Teams whose single member is in a field. */
export const team = new Policy(module1.package, {
    name: "team",
    relations: { member: { subjects: [principal.user] } },
    permissions: {},
});

/** Cells constrained by an agent's allowed row and column interval. */
export const cell = new Policy(module2.package, {
    name: "cell",
    attributes: { row: "number", column: "number", locked: "number" },
    context: {
        "first-row": "number",
        "last-row": "number",
        "first-column": "number",
        "last-column": "number",
    },
    relations: { owner: { subjects: [principal.user] } },
    permissions: {
        read: relation("owner"),
        edit: intersection(
            relation("owner"),
            resource({
                row: {
                    gte: sql.placeholder("first-row"),
                    lte: sql.placeholder("last-row"),
                },
                column: {
                    gte: sql.placeholder("first-column"),
                    lte: sql.placeholder("last-column"),
                },
                locked: 0,
            }),
        ),
    },
});

/** World entities combine team membership with authoritative phase and protection. */
export const entity = new Policy(module3.package, {
    name: "entity",
    attributes: { team: "number", protected: "number" },
    context: { team: "number", phase: "string" },
    relations: { owner: { subjects: [principal.user] } },
    permissions: {
        read: relation("owner"),
        edit: exclusion(
            intersection(
                resource({ team: { eq: sql.placeholder("team") } }),
                context({ phase: "edit" }),
            ),
            resource({ protected: 1 }),
        ),
    },
    elevated: { edit: RECENT },
});

/** Real application records shared by the three compact integration examples. */
export const item = defineTable(
    "example_item",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        owner: text("owner").notNull(),
        parent: text("parent"),
        row: integer("row").notNull(),
        column: integer("column").notNull(),
        locked: integer("locked").notNull(),
        team: integer("team").notNull(),
        protected: integer("protected").notNull(),
    },
    { tree: { id: "id", scope: "scope", parent: "parent" } },
);

/** Groups with relationships as members. */
export const groupTable = defineTable("example_group", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
});

/** Teams keeping their member in a column. */
export const teamTable = defineTable(
    "example_team",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        member: text("member").notNull(),
    },
    { constraints: (teams) => [index("example_team_member").on(teams.member)] },
);

/** Accounts, scope objects living in the universe. */
export const accountTable = defineTable("example_account", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
});

/** Policies a scope sets for itself or hands down to the scopes inside it. */
export const policyTable = defineTable(
    "example_policy",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        mode: text("mode", { enum: ["set", "require"] }).notNull(),
        value: text("value").notNull(),
    },
    { log: {} },
);

/** Spaces, scope objects listed by the account containing them. */
export const spaceTable = defineTable("example_space", {
    id: text("id").primaryKey(),
    account: text("account").notNull(),
});

/** The ancestor index over the items' parent column. */
export const nodeTree = requireTree(item[TABLE].tree);

/** Policies a scope sets for itself or requires of the scopes inside it. */
export const policy = new Policy(module4.package, { name: "policy", permissions: {} });

/** Map application records and the indexed parent tree to access declarations. */
export const mappings: TableMapping[] = [
    {
        policy,
        table: policyTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {},
        inherited: { mode: "require" },
    },
    { policy: group, table: groupTable, id: "id", scope: "scope", attributes: {}, relations: {} },
    {
        policy: team,
        table: teamTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: { member: { column: "member", scope: "universe" } },
    },
    ...[node, cell, entity].map((type): TableMapping => ({
        policy: type,
        table: item,
        id: "id",
        scope: "scope",
        attributes: Object.fromEntries(
            Object.keys(type.definition.attributes).map((name) => [name, name]),
        ),
        relations: {
            owner: { column: "owner", scope: "universe" },
            ...(type === node ? { parent: { column: "parent" } } : {}),
        },
        trees: type === node ? { parent: nodeTree } : {},
    })),
];

/** The mappings of a database that also keeps and writes the scope objects. */
export const homeMappings: TableMapping[] = [
    ...mappings,
    {
        policy: account,
        table: accountTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {},
    },
    { policy: space, table: spaceTable, id: "id", scope: "account", attributes: {}, relations: {} },
];

/** The fixture's policies. */
export const policies = [account, space, group, team, node, cell, entity, policy];

/** The fixture's application and access tables. */
export const fixtureTables = [
    item,
    groupTable,
    teamTable,
    accountTable,
    spaceTable,
    policyTable,
    ...accessTables,
];

/** The fixture database with the application and access tables. */
export const fixtureDatabase = defineDatabase({
    name: "main",
    tables: fixtureTables,
});

/** Application records spanning two isolated scopes. */
export const rows = [
    {
        id: "a",
        scope: "personal",
        owner: "alice",
        parent: null,
        row: 1,
        column: 1,
        locked: 0,
        team: 1,
        protected: 0,
    },
    {
        id: "b",
        scope: "personal",
        owner: "alice",
        parent: "a",
        row: 2,
        column: 1,
        locked: 0,
        team: 1,
        protected: 1,
    },
    {
        id: "c",
        scope: "personal",
        owner: "alice",
        parent: "b",
        row: 3,
        column: 1,
        locked: 1,
        team: 2,
        protected: 0,
    },
    {
        id: "d",
        scope: "work",
        owner: "alice",
        parent: null,
        row: 1,
        column: 1,
        locked: 0,
        team: 1,
        protected: 0,
    },
];

/** Require the tree a table declares. */
function requireTree(declared: Tree | undefined): Tree {
    if (declared === undefined) {
        throw new TypeError("the item table declares no tree");
    }

    return declared;
}

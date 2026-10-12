import { expect, test } from "@destack/test";
import { principal } from "@destack/access";
import { declareState, TABLE } from "@destack/db";
import { defineResourceKind, ResourceDeclaration } from "@destack/resource";
import { schema, toJsonSchema } from "@destack/schema";
import { defineObject, field, type FileBucket, ObjectType } from "../src/index.ts";
import { Chunk } from "../src/text/index.ts";
import { space } from "./fixture/space.ts";
import { comment, folder, label, note, task, taskCopy, taskVersion, team, user } from "./schema.ts";

/** The bucket the sample's file field keeps its files in. */
const files: FileBucket = new ResourceDeclaration(note.package, {
    name: "files",
    kind: "bucket",
    spec: {},
});

/** Boxes a provider provisions. */
const BoxKind = defineResourceKind("box", { spec: schema.object({}) });

/** A space's box: a resource of the box kind. */
const box = defineObject({
    name: "box",
    plural: "boxes",
    scope: space,
    provisioned: { kind: BoxKind },
    fields: {},
}).methods();

/** A type with a field of every builder and modifier, a method of every builder and option, and a constraint of every kind. */
const sample = defineObject({
    name: "sample",
    plural: "samples",
    scope: space,
    fields: {
        title: field.string(schema.string().min(1).max(80)),
        slug: field.string().optional(),
        rank: field.integer().default(3),
        exact: field.bigint(),
        ratio: field.number().sensitive(),
        bytes: field.binary(),
        done: field.boolean().guard({ read: "audit", write: "edit" }),
        due: field.time().personal(),
        kind: field.enum(["draft", "final"]).guardWrite("edit"),
        meta: field.json(schema.object({ weight: schema.number() })),
        attachment: field.file(files),
        author: field.reference(principal.user).caller().relate("author"),
        member: field.subject(principal.user, principal.installation),
        status: field.state({
            initial: "open",
            transitions: { close: { from: ["open"], to: "closed", permission: "edit" } },
        }),
        position: field.fractionalIndex(),
        body: field.text(),
        total: field.count(),
        largest: field.max(),
        note: field.reference(note, { delete: "cascade" }).relate("note"),
        later: field.reference("note", () => note).relate("later"),
        link: field.reference("self", { qualified: true }),
        elsewhere: field.reference(note, { qualified: true }),
    },
    permissions: ["read", "write", "edit", "audit", "tidy"],
    constraints: {
        rank: { kind: "check", condition: { rank: { gte: 0 }, title: { length: { lte: 80 } } } },
        title: { kind: "index", on: ["title"], where: { kind: "draft" }, nullsNotDistinct: true },
        kindTitle: { kind: "unique", on: ["kind", "title"] },
        note: {
            kind: "foreignKey",
            on: ["noteId"],
            references: { object: () => note, on: ["id"] },
            onDelete: "cascade",
        },
    },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    create: method.create("write", {
        fields: ["title", "kind"],
        variants: { kind: { draft: ["rank"], final: [] } },
        input: schema.object({ note: schema.string().exactOptional() }),
        creator: "owner",
    }),
    update: method.update("write", { fields: ["title"], isPredicted: false }),
    updateMany: method.updateMany("write", { fields: ["title"], match: ["kind"] }),
    delete: method.delete("write", { isInternal: true }),
    count: method.query({
        permission: "read",
        input: schema.discriminatedUnion("by", [
            schema.object({ by: schema.literal("kind"), kind: schema.string() }),
            schema.object({ by: schema.literal("title"), prefix: schema.string() }),
        ]),
        output: schema.object({ total: schema.int() }),
    }),
    archive: method.mutation({
        permission: "write",
        prepared: schema.string(),
        audit: { details: schema.object({ reason: schema.string() }) },
        inverse: "restore",
    }),
    tidy: method.mutation({ permission: "tidy", target: "none", isInternal: true }),
}));

/** The test types, in declaration order. */
const TYPES = [
    space,
    user,
    note,
    label,
    team,
    task,
    comment,
    taskVersion,
    taskCopy,
    folder,
    box,
    sample,
];

/** The declarations the test types refer to: the types themselves, two principal kinds and the bucket. */
const KNOWN = [...TYPES, principal.user, principal.installation, files, BoxKind];

test("describe every field and method as its builder calls, and declare the same type from them", () => {
    // describe the type, declare it again and describe that
    const described = sample.describe();
    const declared = ObjectType.of(described, KNOWN);
    expect(declared.describe()).toEqual(described);

    // keep the routes, schemas and tables a manifest and a migration read
    expect([
        proceduresOf(declared),
        declareState(declared.tables, "sqlite"),
        declareState(declared.tables, "postgresql"),
    ]).toEqual([
        proceduresOf(sample),
        declareState(sample.tables, "sqlite"),
        declareState(sample.tables, "postgresql"),
    ]);
});

test("describe fields and methods as the calls written, declarations by reference and schemas as JSON Schema", () => {
    // read the calls of a few fields and methods
    const { definition } = sample.describe();
    const fields = schema.record(schema.string(), schema.json()).parse(definition["fields"]);
    const methods = schema.record(schema.string(), schema.json()).parse(definition["methods"]);
    expect([
        fields["title"],
        fields["rank"],
        fields["done"],
        fields["author"],
        fields["later"],
        fields["attachment"],
        methods["update"],
        methods["tidy"],
    ]).toEqual([
        {
            $field: [
                {
                    builder: "string",
                    arguments: [
                        {
                            $schema: {
                                $schema: "https://json-schema.org/draft/2020-12/schema",
                                type: "string",
                                minLength: 1,
                                maxLength: 80,
                            },
                        },
                    ],
                },
            ],
        },
        {
            $field: [
                { builder: "integer", arguments: [] },
                { builder: "default", arguments: [3] },
            ],
        },
        {
            $field: [
                { builder: "boolean", arguments: [] },
                { builder: "guard", arguments: [{ read: "audit", write: "edit" }] },
            ],
        },
        {
            $field: [
                { builder: "reference", arguments: [{ $policy: principal.user.typeReference }] },
                { builder: "caller", arguments: [] },
                { builder: "relate", arguments: ["author"] },
            ],
        },
        {
            $field: [
                {
                    builder: "reference",
                    arguments: ["note", { $lazy: { $object: note.typeReference } }],
                },
                { builder: "relate", arguments: ["later"] },
            ],
        },
        {
            $field: [
                {
                    builder: "file",
                    arguments: [{ $bucket: { packageId: note.package.id, name: "files" } }],
                },
            ],
        },
        {
            $method: [
                {
                    builder: "update",
                    arguments: ["write", { fields: ["title"], isPredicted: false }],
                },
            ],
        },
        {
            $method: [
                {
                    builder: "mutation",
                    arguments: [{ permission: "tidy", target: "none", isInternal: true }],
                },
            ],
        },
    ]);
});

test("describe every test type as data, and declare the same types, routes and tables from it", () => {
    // describe each type, declare them all again from the platform's declarations alone, and describe them
    const described = TYPES.map((type) => type.describe());
    const declared = ObjectType.ofAll(described, [
        principal.user,
        principal.installation,
        files,
        BoxKind,
    ]);
    expect(declared.map((type) => type.describe())).toEqual(described);

    // keep the routes, schemas and tables a manifest and a migration read
    expect([
        declared.map((type) => proceduresOf(type)),
        declareState(
            declared.flatMap((type) => type.tables),
            "sqlite",
        ),
        declareState(
            declared.flatMap((type) => type.tables),
            "postgresql",
        ),
    ]).toEqual([
        TYPES.map((type) => proceduresOf(type)),
        declareState(
            TYPES.flatMap((type) => type.tables),
            "sqlite",
        ),
        declareState(
            TYPES.flatMap((type) => type.tables),
            "postgresql",
        ),
    ]);
});

test("refuse code, unknown declarations and intrinsic types at their path", () => {
    // a method's inverse written as code, and a reference to a type the reader does not know
    const coded = defineObject({
        name: "coded",
        plural: "codeds",
        scope: space,
        fields: { title: field.string() },
        permissions: ["write"],
    }).methods((method) => ({
        rename: method.mutation({ permission: "write", inverse: () => [] }),
    }));
    expect(() => coded.describe()).toThrow(
        new TypeError(
            "coded.methods.rename.mutation[0].inverse is code, which a description cannot keep",
        ),
    );
    expect(() => ObjectType.of(sample.describe(), [space, principal.user])).toThrow(
        new TypeError(
            `a description refers to bucket ${JSON.stringify([note.package.id, "files"])}, which is not known`,
        ),
    );

    // refuse a lazy reference returning no types
    const empty = defineObject({
        name: "empty",
        plural: "empties",
        scope: space,
        fields: {},
        projections: () => [],
    }).methods();
    expect(() => empty.describe()).toThrow(
        new TypeError("empty.projections returns no object types or policies"),
    );

    // refuse the chunk type, whose table text keeps
    expect(() => Chunk.object([sample]).describe()).toThrow(
        new TypeError("object type chunk maps a table another package keeps"),
    );
});

/** List a type's procedures as clients read them: each route, and its input as JSON Schema. */
function proceduresOf(type: ObjectType) {
    return Object.entries(type.procedures).map(([name, procedure]) => {
        const { route, inputSchema } = procedure["~orpc"];

        return [name, route, inputSchema === undefined ? null : toJsonSchema(inputSchema)];
    });
}

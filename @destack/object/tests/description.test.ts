test("index exactly the code values object's root exports, at the same paths", () => {
    // index the roots' namespaces as a reference, then compare every value either index records
    const reference = new ExportIndex({
        "@destack/object": objectRoot,
        "@destack/access": accessRoot,
    });
    const loaded = ExportIndex.loaded();
    const values = Object.values(objectRoot).flatMap((value): unknown[] => {
        const members: unknown[] =
            typeof value === "object" && value !== null ? Object.values(value) : [];

        return [value, ...members];
    });
    const differing = values
        .filter(
            (value) =>
                JSON.stringify(reference.locate(value)) !== JSON.stringify(loaded.locate(value)),
        )
        .map((value) => [reference.locate(value), loaded.locate(value)]);

    // compare the paths of the declarations and tables both find, in any order
    const paths = (found: readonly unknown[]) =>
        found.map((value) => JSON.stringify(reference.locate(value))).toSorted();
    expect({
        differing,
        declarations: paths(loaded.declarations()),
        tables: paths(loaded.tables()),
    }).toEqual({
        differing: [],
        declarations: paths(reference.declarations()),
        tables: paths(reference.tables()),
    });
});
import {
    claimMethods,
    claimTable,
    invitation,
    defineObject,
    field,
    type FileBucket,
    ObjectDescription,
    ObjectType,
    ObjectTypePool,
} from "../src/index.ts";
import { claim } from "./fixture/claim.ts";
import { ExportIndex } from "../src/object/description.ts";
import * as objectRoot from "../src/index.ts";
import * as accessRoot from "@destack/access";
import { Execution } from "../src/method/execution.ts";
import { expect, test } from "@destack/test";
import { declareState, TABLE } from "@destack/db";
import { principal, through } from "@destack/access";
import { t } from "@destack/locale";
import { defineResourceKind, ResourceDeclaration } from "@destack/resource";
import { nth, schema, toJsonSchema } from "@destack/schema";
import { Chunk } from "../src/text/index.ts";
import { space } from "./fixture/space.ts";
import { comment, folder, label, note, task, taskCopy, taskVersion, team, user } from "./schema.ts";

/** The bucket the sample's file field stores its files in. */
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
});

/** A type with every field builder, method builder and constraint kind. */
const sample = defineObject(
    {
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
            heading: field.message(),
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
            later: field.reference(() => note).relate("later"),
            link: field.object(),
            elsewhere: field.object(note),
        },
        permissions: ["read", "write", "edit", "audit", "tidy"],
        constraints: {
            rank: {
                kind: "check",
                condition: { rank: { gte: 0 }, title: { length: { lte: 80 } } },
            },
            title: {
                kind: "index",
                on: ["title"],
                where: { kind: "draft" },
                nullsNotDistinct: true,
            },
            kindTitle: { kind: "unique", on: ["kind", "title"] },
            note: {
                kind: "foreignKey",
                on: ["noteId"],
                references: { object: () => note, on: ["id"] },
                onDelete: "cascade",
            },
        },
    },
    (method) => ({
        create: method.create("write", {
            fields: ["title", "kind"],
            variants: { kind: { draft: ["rank"], final: [] } },
            input: schema.object({
                note: schema.string().exactOptional(),
                copies: schema.int().positive().default(1),
            }),
            creator: "owner",
        }),
        update: method.update("write", { fields: ["title"], isPredicted: false }),
        updateMany: method.updateMany("write", { fields: ["title"], match: ["kind"] }),
        delete: method.delete("write", { visibility: "package" }),
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
        tidy: method.mutation({ permission: "tidy", target: "scope", visibility: "package" }),
    }),
);

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

/** A mark nested in any object, counted on it. */
const mark = defineObject(
    {
        name: "mark",
        plural: "marks",
        scope: space,
        nested: { in: "any", receive: "mark" },
        fields: {
            /** The mark's label. */
            label: field.string(),
        },
        permissions: { read: through("parent", "read") },
        aggregates: { markCount: { function: "count" } },
    },
    () => ({ update: false }),
);

/** A host the marks attach to. */
const host = defineObject({
    name: "host",
    plural: "hosts",
    scope: space,
    fields: {
        /** The number of marks. */
        markCount: field.count(),
    },
    permissions: ["read", "mark"],
    attachments: [mark.attach({ by: "mark" })],
});
test("describe every field and method as its builder calls, and declare the same type from them", () => {
    // describe the type, declare it again and describe that
    const described = ObjectDescription.of(sample);
    const declared = nth(ObjectTypePool.of(TYPES).declare([described]), 0);
    expect(ObjectDescription.of(declared)).toEqual(described);

    // fill a declared input's default as the written type does
    expect(declared.method("create").input?.parse({})).toEqual({ copies: 1 });

    // read the routes, schemas and tables a manifest and a migration read
    expect([
        readProcedures(declared),
        declareState(declared.tables, "sqlite"),
        declareState(declared.tables, "postgresql"),
    ]).toEqual([
        readProcedures(sample),
        declareState(sample.tables, "sqlite"),
        declareState(sample.tables, "postgresql"),
    ]);
});

test("describe fields and methods as the calls written, types by reference, kinds and buckets as data and schemas as JSON Schema", () => {
    // read the calls of a few fields and methods
    const { definition } = ObjectDescription.of(sample);
    const fields = schema.jsonObject().parse(definition["fields"]);
    const methods = schema.jsonObject().parse(definition["methods"]);
    expect([
        fields["title"],
        fields["rank"],
        fields["done"],
        fields["author"],
        fields["later"],
        fields["attachment"],
        fields["heading"],
        methods["update"],
        methods["tidy"],
        ObjectDescription.of(box).definition["provisioned"],
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
                    arguments: [{ $lazy: { $object: note.typeReference } }],
                },
                { builder: "relate", arguments: ["later"] },
            ],
        },
        {
            $field: [
                {
                    builder: "file",
                    arguments: [{ $bucket: { package: note.package, name: "files", spec: {} } }],
                },
            ],
        },
        { $field: [{ builder: "message", arguments: [] }] },
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
                    arguments: [{ permission: "tidy", target: "scope", visibility: "package" }],
                },
            ],
        },
        {
            kind: {
                $kind: {
                    package: box.package,
                    name: "box",
                    spec: toJsonSchema(BoxKind.spec),
                    state: null,
                    management: "internal",
                },
            },
        },
    ]);
});

test("encode a message field's package, identifier and source text, and refuse plain text", () => {
    // read a message back from its JSON form, and check plain text against the column
    const heading = t`Meet Destack.`;
    const column = sample.table.heading.definition;

    expect({
        decoded: column.fromJson(JSON.parse(JSON.stringify(heading))),
        isText: column.json.safeParse("Meet Destack.").success,
    }).toEqual({
        decoded: { package: note.package.id, id: heading.id, source: "Meet Destack.", values: {} },
        isText: false,
    });
});

test("describe every test type as data, and declare the same types, routes and tables from it", () => {
    // describe each type, declare them all again only from the descriptions, and describe them
    const described = TYPES.map((type) => ObjectDescription.of(type));
    const declared = ObjectTypePool.of([]).declare(described);
    expect(declared.map((type) => ObjectDescription.of(type))).toEqual(described);

    // read the routes, schemas and tables a manifest and a migration read
    expect([
        declared.map((type) => readProcedures(type)),
        declareState(
            declared.flatMap((type) => type.tables),
            "sqlite",
        ),
        declareState(
            declared.flatMap((type) => type.tables),
            "postgresql",
        ),
    ]).toEqual([
        TYPES.map((type) => readProcedures(type)),
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

test("declare described types against each other, never against the pool's types of the same names", () => {
    // describe a host attaching marks, the host first, into a pool that already knows both
    const declared = ObjectTypePool.of([space, mark, host]).declare([
        ObjectDescription.of(host),
        ObjectDescription.of(mark),
    ]);

    // read one complete table state from the declared types alone, the host's mark dependents among them
    expect(
        declareState(
            declared.flatMap((type) => type.tables),
            "sqlite",
        ),
    ).toEqual(declareState([...host.tables, ...mark.tables], "sqlite"));
});

test("refuse code and unknown declarations at their path", () => {
    // a method's inverse written as code, and a reference to a type the reader does not know
    const coded = defineObject(
        {
            name: "coded",
            plural: "codeds",
            scope: space,
            fields: { title: field.string() },
            permissions: ["write"],
        },
        (method) => ({
            rename: method.mutation({ permission: "write", inverse: () => [] }),
        }),
    );
    expect(() => ObjectDescription.of(coded)).toThrow(
        new TypeError(
            "a description cannot store coded.methods.rename.mutation[0].inverse: no loaded package exports it",
        ),
    );
    expect(() => ObjectTypePool.of([space]).declare([ObjectDescription.of(sample)])).toThrow(
        new TypeError(
            `a description refers to unknown object type ${JSON.stringify([note.package.id, "note"])}`,
        ),
    );
});

test("describe a type over another package's table by the table's name, and declare it over that same table", () => {
    // describe the chunk type, which maps the text module's table
    const chunked = Chunk.object([sample]);
    const description = ObjectDescription.of(chunked);
    const declared = ObjectTypePool.of([sample, space]).declare([description]);

    // set the table in the description, and declare the type over the very table
    expect({
        table: schema
            .looseObject({ mapping: schema.looseObject({ table: schema.unknown() }) })
            .parse(description.definition).mapping.table,
        isSameTable: declared[0]?.table === chunked.table,
    }).toEqual({ table: { $table: chunked.table[TABLE].sqlName }, isSameTable: true });
});

test("describe code another package exports by its export path, and declare the type with that very code", () => {
    // describe the claims, whose methods and table are object's exports
    const description = ObjectDescription.of(claim);
    const declared = ObjectTypePool.of([]).declare([description]);

    // write the methods as export paths and the table by name, and declare the very methods and table
    const written = schema
        .looseObject({
            mapping: schema.looseObject({ table: schema.unknown() }),
            methods: schema.unknown(),
        })
        .parse(description.definition);
    expect({
        table: written.mapping.table,
        methods: written.methods,
        isSameReserve: declared[0]?.methods["reserve"] === claimMethods.reserve,
        isSameTable: declared[0]?.table === claimTable,
    }).toEqual({
        table: { $table: claimTable[TABLE].sqlName },
        methods: Object.fromEntries(
            Object.keys(claimMethods).map((name) => [
                name,
                { $export: { package: "@destack/object", path: ["claimMethods", name] } },
            ]),
        ),
        isSameReserve: true,
        isSameTable: true,
    });
});

test("describe an exported type's handled method by its path through the type, resolving to that very method", () => {
    // describe object's invitations, whose create method runs object's code
    const written = schema
        .looseObject({ methods: schema.record(schema.string(), schema.unknown()) })
        .parse(ObjectDescription.of(invitation).definition).methods["create"];
    const path = { package: "@destack/object", path: ["invitation", "methods", "create"] };

    // write the method as its path, which resolves to the type's method
    expect({
        written,
        isSame: ExportIndex.loaded().resolve(path) === invitation.methods.create,
    }).toEqual({
        written: { $export: path },
        isSame: true,
    });
});

test("refuse a handled method no loaded package exports, and an export path the index never recorded", () => {
    // declare notes whose update runs code that is no package's export
    const handled = defineObject(
        {
            name: "handled",
            plural: "handleds",
            scope: space,
            fields: { title: field.string() },
            permissions: ["write"],
        },
        (method) => ({
            update: Execution.handle(
                method.update("write", { fields: ["title"] }),
                async (call) => call.target,
            ),
        }),
    );

    // refuse describing it, and refuse resolving a path no module exports
    expect(() => ObjectDescription.of(handled)).toThrow(
        new TypeError(
            "handled.methods.update has a handler; attach it with .implement({ methods })",
        ),
    );
    expect(() =>
        ExportIndex.loaded().resolve({ package: "@destack/object", path: ["nothing"] }),
    ).toThrow("the export @destack/object#nothing");
});

/** List a type's procedures as clients read them: each route, and its input as JSON Schema. */
function readProcedures(type: ObjectType) {
    return Object.entries(type.procedures).map(([name, procedure]) => {
        const { route, inputSchema } = procedure["~orpc"];

        return [name, route, inputSchema === undefined ? null : toJsonSchema(inputSchema)];
    });
}

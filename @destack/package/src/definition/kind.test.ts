import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Declaration } from "../graph/declaration.ts";
import { defineKind, Kind } from "./kind.ts";
import { PackageVersion } from "./package.ts";

/** The package declaring the settings and the command. */
const owner = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-00000000000a",
    name: "@example/notes",
    version: "2026.9.0",
});

/** Declare a symbol of the package as a kind of its own, with a description. */
function declared(kind: string, name: string, description: Record<string, unknown>) {
    const symbol = `${owner.id}/src/index.ts#${name}`;

    return Declaration.at(Declaration.shape.symbol.parse(symbol), {
        kind: Declaration.shape.kind.parse(kind),
        kindPackageId: owner.id,
        name,
        description: Declaration.shape.description.parse(description),
    });
}

test("select a kind's declarations by its package and name, parsing their descriptions by its schema", () => {
    // declare two settings and a command of the package's own kinds
    const declarations = [
        declared("setting", "theme", { title: "Theme" }),
        declared("setting", "locale", { title: "Locale" }),
        declared("command", "archive", { title: "Archive" }),
    ];
    const setting = new Kind(
        owner.id,
        "setting",
        schema.object({ title: schema.string() }),
        undefined,
    );
    const strict = new Kind(
        owner.id,
        "setting",
        schema.object({ order: schema.number() }),
        undefined,
    );

    // select the settings with their titles, and refuse a description the kind's schema refuses
    expect({
        titles: setting.select(declarations).map((each) => each.description.title),
        refused: issues(() => strict.select(declarations)),
    }).toEqual({
        titles: ["Theme", "Locale"],
        refused: [
            ["invalid_type", "order"],
            ["unrecognized_keys", ""],
        ],
    });
});

/** Read the codes and paths of the schema issues a selection throws. */
function issues(select: () => unknown): (readonly [string, string])[] {
    try {
        select();
    } catch (error) {
        if (error instanceof schema.Error) {
            return error.issues.map((issue) => [issue.code, issue.path.join(".")] as const);
        }
        throw error;
    }
    throw new Error("the selection threw nothing");
}

test("tell a kind reconciling toward a desired state from one without", () => {
    // declare a kind with a desired state and one without
    const module = { package: owner };
    const stateful = defineKind(
        {
            name: "table",
            provisioned: {
                spec: schema.object({}),
                state: schema.object({ columns: schema.number() }),
            },
        },
        module,
    );
    const stateless = defineKind(
        { name: "link", provisioned: { spec: schema.object({}) } },
        module,
    );

    expect([stateful.isStateful(), stateless.isStateful()]).toEqual([true, false]);
});

test("tell a kind outside Destack from one Destack creates", () => {
    const module = { package: owner };
    const api = defineKind(
        { name: "api", provisioned: { spec: schema.object({}), management: "external" } },
        module,
    );
    const link = defineKind({ name: "link", provisioned: { spec: schema.object({}) } }, module);

    expect([api.provisioned.management, link.provisioned.management]).toEqual([
        "external",
        "internal",
    ]);
});

import { principal, relation } from "@destack/access";
import { Expression } from "@destack/db";
import { argument, plural, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { defineObject, field } from "../index.ts";
import { space } from "../test/space.ts";
import { compareObject, describeObject } from "./object.ts";

/** The release under comparison. */
const RELEASE = "2026.9.0";

/** A card type at a release: its title field's name and its conversions. */
const card = (
    title: "title" | "name",
    convert: Readonly<Record<number, Readonly<Record<string, Expression>>>>,
) =>
    defineObject(
        {
            name: "card",
            plural: "cards",
            title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
            scope: space,
            convert,
            fields: {
                owner: field.reference(principal.user).caller(),
                [title]: field.string(schema.string().min(1)),
            },
            access: {
                permissions: { read: relation("owner"), write: relation("owner") },
            },
        },
        (method) => ({
            create: method.create("write"),
            update: method.update("write"),
        }),
    );

/** A release's entry of a card type, its description read back from JSON. */
const entry = (type: ReturnType<typeof card>, version: string) => ({
    description: schema.jsonObject().parse(JSON.parse(JSON.stringify(describeObject(type)))),
    symbol: { package: { ...type.package, version } },
    capabilities: [],
});

/** The step an output breaking earlier readers plans. */
const incompatible = (method: string) => ({
    action: "update",
    detail: "incompatible values: earlier readers stay on their release",
    risk: "backward-incompatible",
    target: `method/${method}/output`,
});

/** Plan a release's steps from the earlier one, else read why no plan reaches it. */
const outcome = (later: ReturnType<typeof card>) => {
    try {
        return compareObject(entry(card("title", {}), "2026.8.0"), entry(later, RELEASE)).steps;
    } catch (error) {
        // read the refusal's message
        if (!(error instanceof Error)) {
            throw new TypeError("comparing types failed with a value that is no error", {
                cause: error,
            });
        }

        return error.message;
    }
};

test("plan an object type's method changes between releases, converting renamed inputs and refusing unconverted ones", () => {
    // rename the title with a conversion, without one, and change nothing, outputs carrying the new name
    const converted = card("name", { 2: { name: Expression.column("title") } });
    const unconverted = card("name", {});
    const unchanged = card("title", {});

    expect({
        unchanged: outcome(unchanged),
        converted: outcome(converted),
        unconverted: outcome(unconverted),
    }).toEqual({
        unchanged: [],
        converted: [
            {
                action: "convert",
                detail: "convert values to 2026.9.0",
                risk: "fallible",
                target: "method/create/input",
            },
            incompatible("create"),
            {
                action: "convert",
                detail: "convert values to 2026.9.0",
                risk: "fallible",
                target: "method/update/input",
            },
            ...["update", "get", "list", "detach", "attach"].map((method) => incompatible(method)),
        ],
        unconverted:
            "cannot reach the desired state: method/create/input: declare a conversion for 2026.9.0; method/update/input: declare a conversion for 2026.9.0",
    });
});

import { Expression } from "@destack/db";
import { expect, test } from "@destack/test";
import { ObjectTable } from "./table.ts";

test("read a removed field of a conversion by its column, a kept one by its name, leaving renames out", () => {
    // convert a label from a removed task kind and the kept title, and rename the old summary
    const label = Expression.case(
        Expression.column("taskKind"),
        [{ when: "bug", then: Expression.literal("Bug") }],
        Expression.column("title"),
    );
    const convert = { 2: { label, description: Expression.column("summary") } };
    const columns = { title: {}, label: {}, description: {} };

    // expect the removed field read by its snake-cased column and the rename left to the table's moves
    expect(ObjectTable.conversions({ convert }, columns)).toEqual({
        2: {
            label: Expression.case(
                Expression.column("task_kind"),
                [{ when: "bug", then: Expression.literal("Bug") }],
                Expression.column("title"),
            ),
        },
    });
});

import { expect, test } from "@destack/test";
import type { JsonValue } from "../json/json.ts";
import { Expression } from "./expression.ts";

test("upgrade partial records through each later release, leaving fields computed from absent fields absent", () => {
    // rename title to name in 2026.9.0, and default a new limit in 2026.10.0
    const conversions = {
        "2026.9.0": { name: Expression.column("title") },
        "2026.10.0": {
            limit: Expression.coalesce(Expression.column("limit"), Expression.literal(50)),
        },
    };
    const convert = (record: Record<string, JsonValue>, from: string) =>
        Expression.upgrade(conversions, record, from, "2026.10.0");

    // convert a full record, a partial one, an explicit null, and a record of the latest release
    expect([
        convert({ title: "Plan" }, "2026.8.0"),
        convert({ id: "a" }, "2026.8.0"),
        convert({ title: null }, "2026.8.0"),
        convert({ name: "Plan", limit: 5 }, "2026.10.0"),
    ]).toEqual([
        { title: "Plan", name: "Plan", limit: 50 },
        { id: "a", limit: 50 },
        { title: null, name: null, limit: 50 },
        { name: "Plan", limit: 5 },
    ]);
});

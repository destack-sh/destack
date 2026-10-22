import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { archive, item, list } from "../aggregate/test/fixture.ts";
import { post, remark } from "../dependent/test/fixture.ts";
import { TABLE } from "../table/index.ts";
import { declareState } from "./state.ts";

/** A package that declares none of the fixture's tables. */
const OTHER = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000e1");

test("describe the aggregates of the described tables alone, a replica's too, requiring their tables by name", () => {
    // place another package's aggregated table without describing its aggregates
    const placed = declareState([item], "sqlite", { package: OTHER });

    // describe a replica's aggregates, which its predictions maintain
    const replicated = declareState([item, list], "sqlite", { isReplica: true }).map((state) =>
        (state.aggregates ?? []).map((aggregate) => aggregate.column),
    );

    // describe an aggregate declared through another object of the same table
    const described = declareState([archive, item, list], "sqlite").map((state) => [
        state.table.name,
        state.aggregates?.map((aggregate) => [aggregate.table, aggregate.column]) ?? [],
    ]);

    // refuse an aggregate into a table the database lacks
    expect(() => declareState([item], "sqlite")).toThrow(
        new TypeError(
            "aggregate of aggregate_item into aggregate_list references undeclared table aggregate_list",
        ),
    );
    expect({ placed, replicated, described }).toEqual({
        placed: [],
        replicated: [["items", "done", "points", "largest"], []],
        described: [
            ["destack__db__aggregate_archive", []],
            [
                "destack__db__aggregate_item",
                [
                    ["destack__db__aggregate_archive", "done"],
                    ["destack__db__aggregate_list", "items"],
                    ["destack__db__aggregate_list", "done"],
                    ["destack__db__aggregate_list", "points"],
                    ["destack__db__aggregate_list", "largest"],
                ],
            ],
            ["destack__db__aggregate_list", []],
        ],
    });
});

test("describe a replicated table's dependents only where the database has their table, refusing a local table's missing one", () => {
    // replicate posts without their remarks, then beside them
    const replicated = new Set([post[TABLE].sqlName]);
    const alone = declareState([post], "sqlite", { replicated });
    const beside = declareState([post, remark], "sqlite", { replicated });

    // refuse local posts without their remarks
    expect(() => declareState([post], "sqlite")).toThrow(
        new TypeError("dependents of dependent_post reference undeclared table dependent_remark"),
    );
    expect(
        [alone, beside].map((states) => states.map((state) => state.dependents?.length ?? 0)),
    ).toEqual([[0], [1, 0]]);
});

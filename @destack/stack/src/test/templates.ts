import { BORN_SPACE, RESIDENCY_HOME, TestSpace } from "./fixture.ts";
import { PARTITION_TEMPLATE } from "./partition.ts";
import { ROUTE_TEMPLATE } from "./route.ts";
import { gauge, meter, note, sharedNote } from "./schema.ts";

/** The templates stack's tests copy, which the run writes once. */
export const templates = [
    TestSpace.template({}, BORN_SPACE),
    TestSpace.template({}, RESIDENCY_HOME),
    ...[gauge, meter, note, sharedNote].map((type) =>
        TestSpace.template(
            { packages: [{ objects: { [type.name]: type }, tables: [] }] },
            BORN_SPACE,
        ),
    ),
    ROUTE_TEMPLATE,
    PARTITION_TEMPLATE,
];

import { spaceTables } from "../src/stack/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { accessRole } from "@destack/access";
import * as sqlite from "@destack/db/bun";
import { Stack } from "@destack/object/server";
import { identifier } from "@destack/schema";
import { role } from "../src/server/index.ts";
import { installation } from "../src/object/index.ts";

/** The space the roles belong to, and the stack declaring them. */
const ids = {
    space: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    installation: identifier("installation").parse(
        "installation-01996ab0-0000-7000-8000-000000000003",
    ),
    package: identifier("package").parse("package-01996ab0-0000-7000-8000-000000000007"),
};

test("declare a stack's roles once, and reapply them unchanged without writing", async () => {
    const database = await sqlite.connect(":memory:", spaceTables);
    onTestFinished(() => database.close());
    await database.migrate(spaceTables);

    // declare an editor role granting installation reads
    const apply = () =>
        Stack.apply({
            database,
            objects: [role],
            manager: { installationId: ids.installation, packageId: ids.package },
            scope: ids.space,
            document: {
                roles: {
                    editor: {
                        description: "Reads installations",
                        permissions: [installation.permission("read")],
                    },
                },
            },
        });
    expect(await apply()).toEqual({
        steps: [{ action: "create", target: "role/editor", risk: "safe", detail: "declare" }],
    });
    const [created] = await database.select().from(accessRole);

    // reapply the same roles without changing or rewriting them
    expect(await apply()).toEqual({ steps: [] });
    expect(await database.select().from(accessRole)).toEqual([created]);
});

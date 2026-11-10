import { testSimulation } from "@destack/test/world";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { InstallationId, principal } from "./principal.ts";

test("derive each space's installation ids by name, the same every time", async () => {
    // name installations in two spaces
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("access");
    const space = principal.space.reference(
        Scope.universe.id,
        "space-01996ab0-0000-7000-8000-0000000000a1",
    );
    const other = principal.space.reference(
        Scope.universe.id,
        "space-01996ab0-0000-7000-8000-0000000000a2",
    );
    const named = [
        await InstallationId.of(space, "forge", crypto),
        await InstallationId.of(space, "forge", crypto),
        await InstallationId.of(space, "notes", crypto),
        await InstallationId.of(other, "forge", crypto),
    ];

    // name one installation per space and name, each a UUIDv8 installation identifier
    expect(named).toEqual([
        "installation-c6412782-d076-82c0-9a4f-a332d35320b7",
        "installation-c6412782-d076-82c0-9a4f-a332d35320b7",
        "installation-2bc6d84c-22ef-86a6-ac28-6c77e89bb3c7",
        "installation-75f63512-6025-8ca0-873b-f90beda5e609",
    ]);
});

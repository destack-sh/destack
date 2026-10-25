import { testSimulation } from "@destack/test/world";
import { PackageId } from "@destack/package";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { InstallationId, principal } from "./principal.ts";

/** A package a scope's base installation runs. */
const STACK = PackageId.parse("package-01996ab0-0000-7000-8000-000000000001");

/** Another package. */
const FORGE = PackageId.parse("package-01996ab0-0000-7000-8000-000000000002");

test("name each scope's base installation of a package and its declared installations apart, the same every time", async () => {
    // name installations in the universe and in a space
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("access");
    const space = principal.space.reference(
        Scope.universe.id,
        "space-01996ab0-0000-7000-8000-0000000000a1",
    );
    const named = [
        await InstallationId.base(Scope.universe, STACK, crypto),
        await InstallationId.base(Scope.universe, STACK, crypto),
        await InstallationId.base(space, STACK, crypto),
        await InstallationId.base(space, FORGE, crypto),
        await InstallationId.declared(space, "forge", crypto),
    ];

    // name one installation per scope and package or name, each a UUIDv8 installation identifier
    expect(named).toEqual([
        "installation-80b408be-76e7-8a29-9f28-28e197dfe7f6",
        "installation-80b408be-76e7-8a29-9f28-28e197dfe7f6",
        "installation-09835d9a-c231-80dd-80b6-28f94e0a336a",
        "installation-950fbc12-0095-83b7-8e74-b368806f598b",
        "installation-cc140a49-3002-8494-9d08-b92a64cf9fc2",
    ]);
});

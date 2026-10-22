import { expect, test } from "@destack/test";
import { PackageId } from "../definition/package.ts";
import { Declaration } from "./declaration.ts";
import { Moniker } from "./moniker.ts";

/** The package defining the declared kinds. */
const KINDS = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a1");

/** The symbol a workflow's constructor runs at. */
const SYMBOL = Moniker.of({
    packageId: "package-01996ab0-0000-7000-8000-0000000000a2",
    module: "src/notification.ts",
    name: "deliver",
});

test("name each declaration at a symbol by its kind and name, and tell its derived members apart", () => {
    // declare a workflow's run and step objects at one symbol, each deriving a get method
    const declared = { kindPackageId: KINDS, description: {} };
    const derived = { kind: "method", name: "get", description: {} };
    const run = Declaration.at(SYMBOL, { ...declared, kind: "object", name: "deliver-run" });
    const step = Declaration.at(SYMBOL, { ...declared, kind: "object", name: "deliver-step" });
    const members = [Declaration.member(run, derived), Declaration.member(step, derived)];

    // name the objects and their members apart, each member under its object
    expect(
        [run, step, ...members].map((declaration) => [
            declaration.moniker,
            Declaration.isMember(declaration),
        ]),
    ).toEqual([
        [`${SYMBOL}:object:deliver-run`, false],
        [`${SYMBOL}:object:deliver-step`, false],
        [`${SYMBOL}:object:deliver-run.get:method`, true],
        [`${SYMBOL}:object:deliver-step.get:method`, true],
    ]);
});

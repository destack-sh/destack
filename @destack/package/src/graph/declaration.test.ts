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
    // declare a workflow's run and step objects at one symbol, and a procedure the run derives
    const declared = { kindPackageId: KINDS, description: {} };
    const run = Declaration.at(SYMBOL, { ...declared, kind: "object", name: "deliver-run" });
    const step = Declaration.at(SYMBOL, { ...declared, kind: "object", name: "deliver-step" });
    const cancel = Declaration.member(run, { kind: "procedure", name: "cancel", description: {} });

    // name both objects apart, and only the procedure a member
    expect(
        [run, step, cancel].map((declaration) => [
            declaration.moniker,
            Declaration.isMember(declaration),
        ]),
    ).toEqual([
        [`${SYMBOL}:object:deliver-run`, false],
        [`${SYMBOL}:object:deliver-step`, false],
        [`${SYMBOL}.cancel:procedure`, true],
    ]);
});

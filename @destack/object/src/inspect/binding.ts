import { graph } from "@destack/package";
import { schema } from "@destack/schema";
import {
    InstallationKind,
    type InstallationBinding,
    type InstallationBindingDescription,
} from "../declare/binding.ts";
import { describeProcedures } from "./procedure.ts";

/** Describe an installation binding for the package manifest. */
export function describeInstallationBinding(
    binding: InstallationBinding,
): InstallationBindingDescription {
    return InstallationKind.description.parse({
        name: binding.name,
        kind: binding.kind,
        spec: binding.spec,
    });
}

/** Bind an installation binding to each procedure of the package it calls. */
export function installationBindingSymbols(binding: InstallationBinding): graph.MemberSymbol[] {
    const { package: packageId } = describeInstallationBinding(binding).spec;
    const called = describeProcedures(binding.handle.router).map((procedure) =>
        procedure.name.join("."),
    );

    return schema.array(graph.MemberSymbol).parse([
        {
            relationships: called.map((name) => ({
                kind: "binds",
                symbol: { packageId, kind: "procedure", name },
            })),
        },
    ]);
}

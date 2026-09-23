import { defineSetting } from "@destack/setting/declare";
import { schema } from "@destack/schema";
import { Package } from "@destack/package";
import definition from "../../destack.json" with { type: "json" };
import metadata from "../../package.json" with { type: "json" };

/** Shared appearance selected personally, per app or per device. */
export const appearance = defineSetting({
    package: Package.parse({ id: definition.id, name: metadata.name, version: metadata.version }),
    name: "appearance",
    title: "Appearance",
    description: "Use the system appearance or select a light or dark interface.",
    schema: schema.enum(["system", "light", "dark"]),
    default: "system",
    scope: "user",
    overrides: ["package", "space", "installation", "device"],
    apply: "immediate",
});

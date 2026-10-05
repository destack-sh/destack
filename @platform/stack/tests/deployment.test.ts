import { resolve } from "node:path";
import { expect, test } from "@destack/test";
import { Deployment, ROOT } from "../src/deployment/index.ts";

test("select each tier's root and keep each deployment's remote state key", () => {
    // read the root, state key and data directory of every deployment kind
    const selected = [["shared"], ["production", "universe"], ["development", "eu"]].map(
        (selection) => {
            const { name, directory, state, cache } = new Deployment(selection);

            return { name, directory, state, cache };
        },
    );

    // keep the universe state at the key its resources were first applied under
    expect(selected).toEqual([
        {
            name: "shared",
            directory: resolve(ROOT, "src/shared"),
            state: "platform.tfstate",
            cache: resolve(ROOT, ".terraform/shared"),
        },
        {
            name: "production/universe",
            directory: resolve(ROOT, "src/universe"),
            state: "production/global.tfstate",
            cache: resolve(ROOT, ".terraform/production/universe"),
        },
        {
            name: "development/eu",
            directory: resolve(ROOT, "src/residency"),
            state: "development/eu.tfstate",
            cache: resolve(ROOT, ".terraform/development/eu"),
        },
    ]);
});

test("refuse a scope outside the universe and the residencies", () => {
    expect(() => new Deployment(["production", "global"])).toThrow(
        new Error("select shared, or development|production followed by universe|eu|us"),
    );
});

import { resolve } from "node:path";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { Root, ROOT, TOFU_VARIABLES, WRANGLER_VARIABLES } from "./index.ts";

test("select each tier's root and keep each deployment's remote state key", () => {
    // read the root, state key and data directory of every deployment kind
    const environment = testSimulation().environment("operator");
    const selected = [["global"], ["production", "universe"], ["production", "eu"]].map(
        (selection) => {
            const { name, directory, state, cache } = new Root(selection, environment);

            return { name, directory, state, cache };
        },
    );

    // select each root's directory, state key and cache
    expect(selected).toEqual([
        {
            name: "global",
            directory: resolve(ROOT, "src/deployment/roots/global"),
            state: "global.tfstate",
            cache: resolve(ROOT, ".terraform/global"),
        },
        {
            name: "production/universe",
            directory: resolve(ROOT, "src/deployment/roots/universe"),
            state: "production/universe.tfstate",
            cache: resolve(ROOT, ".terraform/production/universe"),
        },
        {
            name: "production/eu",
            directory: resolve(ROOT, "src/deployment/roots/residency"),
            state: "production/eu.tfstate",
            cache: resolve(ROOT, ".terraform/production/eu"),
        },
    ]);
});

test("refuse a scope outside the universe and the residencies", () => {
    const environment = testSimulation().environment("operator");
    expect(() => new Root(["production", "global"], environment)).toThrow(
        new Error("select global, or production followed by universe|eu|us"),
    );
});

/** An operator's process: their host, the tools' own variables, and secrets no tool reads. */
const OPERATOR = {
    HOME: "/home/operator",
    PATH: "/usr/bin",
    CI: "true",
    TF_LOG: "info",
    AWS_ACCESS_KEY_ID: "AKIA",
    PLANETSCALE_SERVICE_TOKEN: "pscale",
    WRANGLER_LOG: "debug",
    CLOUDFLARE_ACCOUNT_ID: "account",
    CLOUDFLARE_COMPANY_API_TOKEN: "company",
    BW_SESSION: "vault",
};

test("run each tool with the host's variables and its own, leaving out the company's secrets", () => {
    expect({
        tofu: Root.inherited(OPERATOR, TOFU_VARIABLES),
        wrangler: Root.inherited(OPERATOR, WRANGLER_VARIABLES),
    }).toEqual({
        tofu: {
            HOME: "/home/operator",
            PATH: "/usr/bin",
            CI: "true",
            TF_LOG: "info",
            AWS_ACCESS_KEY_ID: "AKIA",
            PLANETSCALE_SERVICE_TOKEN: "pscale",
        },
        wrangler: {
            HOME: "/home/operator",
            PATH: "/usr/bin",
            CI: "true",
            WRANGLER_LOG: "debug",
            CLOUDFLARE_ACCOUNT_ID: "account",
        },
    });
});

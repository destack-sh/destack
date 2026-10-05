import { resolve } from "node:path";
import { PLATFORM, Placement } from "../placement/index.ts";
import { type Environment, WorkerScript } from "../cloudflare/worker/script.ts";

/** The stack package directory. */
export const ROOT = resolve(import.meta.dirname, "../..");

/** The remote state name of the universe tier, which keeps the name its state was first written under. */
const UNIVERSE_STATE = "global";

/** An independently managed infrastructure deployment. */
export class Deployment {
    /** The deployment path within its environment. */
    readonly name: string;
    /** The OpenTofu root directory. */
    readonly directory: string;
    /** The variable arguments: the deployment's variable file, and the processes its placement runs there. */
    readonly variables: readonly string[];
    /** The remote state object key. */
    readonly state: string;
    /** The isolated OpenTofu working directory. */
    readonly cache: string;

    /** Select shared infrastructure or one environment and scope. */
    constructor(selection: string[]) {
        // select the shared root, or one environment and scope
        const [environment, scope] = selection;
        if (selection.length === 1 && environment === "shared") {
            this.name = "shared";
            this.directory = resolve(ROOT, "src/shared");
            this.state = "platform.tfstate";
        } else if (
            selection.length === 2 &&
            (environment === "development" || environment === "production") &&
            (scope === "universe" || scope === "eu" || scope === "us")
        ) {
            this.name = `${environment}/${scope}`;
            this.directory = resolve(ROOT, "src", scope === "universe" ? "universe" : "residency");
            this.state = `${environment}/${scope === "universe" ? UNIVERSE_STATE : scope}.tfstate`;
        } else {
            throw new Error("select shared, or development|production followed by universe|eu|us");
        }

        // derive paths after validating the deployment selection, and the processes placed in a scope
        const file = `-var-file=${resolve(ROOT, "deployment", `${this.name}.tfvars`)}`;
        this.variables =
            environment === "shared" || scope === undefined
                ? [file]
                : [file, Deployment.#processes(environment, scope)];
        this.cache = resolve(ROOT, ".terraform", this.name);
    }

    /** Name the processes the placement runs in a scope with the DNS names their Workers' routes answer. */
    static #processes(environment: Environment, scope: string): string {
        const processes = Placement.spread(PLATFORM).processes.filter((placed) =>
            scope === "universe"
                ? placed.residency === undefined
                : placed.residency?.code === scope,
        );
        const value = Object.fromEntries(
            processes.map((placed) => [
                placed.name,
                {
                    workloads: placed.packages.map((entry) => entry.workload.name),
                    names: new WorkerScript(placed, environment).names,
                },
            ]),
        );

        return `-var=processes=${JSON.stringify(value)}`;
    }
}

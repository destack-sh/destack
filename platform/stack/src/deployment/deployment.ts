import { resolve } from "node:path";

/** The stack package directory. */
export const ROOT = resolve(import.meta.dirname!, "../..");

/** An independently managed infrastructure deployment. */
export class Deployment {
    /** The deployment path within its environment. */
    readonly name: string;
    /** The OpenTofu root directory. */
    readonly directory: string;
    /** The deployment variable file. */
    readonly variables: string;
    /** The remote state object key. */
    readonly state: string;
    /** The isolated OpenTofu working directory. */
    readonly cache: string;

    /** Select shared infrastructure or one environment and scope. */
    constructor(selection: string[]) {
        const [environment, scope] = selection;
        if (selection.length === 1 && environment === "shared") {
            this.name = "shared";
            this.directory = resolve(ROOT, "src/shared");
            this.state = "platform.tfstate";
        } else if (
            selection.length === 2 &&
            ["development", "production"].includes(environment) &&
            ["global", "eu", "us"].includes(scope)
        ) {
            this.name = `${environment}/${scope}`;
            this.directory = resolve(ROOT, "src", scope === "global" ? "global" : "regional");
            this.state = `${this.name}.tfstate`;
        } else {
            throw new Error("select shared, or development|production followed by global|eu|us");
        }

        // derive paths after validating the deployment selection
        this.variables = resolve(ROOT, "deployment", `${this.name}.tfvars`);
        this.cache = resolve(ROOT, ".terraform", this.name);
    }
}

import { mkdir, mkdtemp } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { Deployment, ROOT } from "../deployment/index.ts";
import { credentials } from "../credential/index.ts";

/** Check configuration or operate one explicit deployment. */
export async function run(arguments_: string[]): Promise<void> {
    // split the action from its deployment selection
    const [action, ...selection] = arguments_;

    // validate each root without opening remote state
    if (action === "check" || action === "format") {
        if (selection.length) {
            throw new Error(`${action} takes no deployment arguments.`);
        }

        // format each source directory once
        invoke(
            ["fmt", ...(action === "check" ? ["-check"] : []), "-recursive", "src", "deployment"],
            { ...process.env },
        );
        if (action === "format") {
            return;
        }

        // validate every deployment with its own variables
        for (const selection of [
            ["shared"],
            ["development", "global"],
            ["development", "eu"],
            ["development", "us"],
            ["production", "global"],
            ["production", "eu"],
            ["production", "us"],
        ]) {
            const deployment = new Deployment(selection);
            const directory = deployment.directory;
            const environment = {
                ...process.env,
                TF_DATA_DIR: resolve(ROOT, ".terraform", "check", deployment.name),
            };
            invoke(
                [
                    `-chdir=${directory}`,
                    "init",
                    "-backend=false",
                    "-input=false",
                    "-lockfile=readonly",
                    `-var-file=${deployment.variables}`,
                ],
                environment,
            );
            invoke(
                [`-chdir=${directory}`, "validate", `-var-file=${deployment.variables}`],
                environment,
            );
        }

        return;
    }

    // select deployment-specific state, plans and credentials
    if (!["init", "diff", "deploy"].includes(action)) {
        throw new Error("use check, format, init, diff, or deploy");
    }
    const deployment = new Deployment(selection);
    const environment = {
        ...process.env,
        ...(await credentials()),
        TF_DATA_DIR: deployment.cache,
    };
    await mkdir(deployment.cache, { recursive: true, mode: 0o700 });
    const directory = `-chdir=${deployment.directory}`;
    invoke(
        [
            directory,
            "init",
            "-input=false",
            "-lockfile=readonly",
            `-var-file=${deployment.variables}`,
            `-backend-config=key=${deployment.state}`,
        ],
        environment,
    );

    // apply only the reviewed plan for this deployment
    if (action !== "init") {
        const planDirectory = await mkdtemp(resolve(deployment.cache, "plan-"));
        const plan = resolve(planDirectory, "deployment.tfplan");
        invoke(
            [
                directory,
                "plan",
                "-input=false",
                "-lock-timeout=60s",
                `-var-file=${deployment.variables}`,
                `-out=${plan}`,
            ],
            environment,
        );
        if (action === "deploy") {
            if (!confirm(`Apply ${deployment.name}?`)) {
                throw new Error("deployment cancelled");
            }
            invoke([directory, "apply", "-input=false", "-lock-timeout=60s", plan], environment);
        }
    }
}

/** Execute OpenTofu and preserve command failures. */
function invoke(arguments_: string[], environment: NodeJS.ProcessEnv): void {
    // run OpenTofu in the stack root with inherited output
    const command = spawnSync("tofu", arguments_, {
        cwd: ROOT,
        stdio: "inherit",
        env: environment,
    });
    if (command.error) {
        throw command.error;
    }
    if (command.signal) {
        throw new Error(`OpenTofu terminated by ${command.signal}.`);
    }
    if (command.status !== 0) {
        throw new Error(`OpenTofu exited with ${command.status}.`);
    }
}

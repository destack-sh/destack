import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { sql } from "@destack/db";
import * as postgresql from "@destack/db/postgres";
import { schema } from "@destack/schema";
import { Deployment, ROOT } from "../deployment/index.ts";
import { PLATFORM, Placement } from "../placement/index.ts";
import { serveUniverse } from "../universe/index.ts";
import {
    credentials,
    type DeploymentOutputs,
    type Environment,
    WorkerScript,
} from "../cloudflare/index.ts";

/** The identifier a dry run binds where a deployment's state holds a real one: 32 hexadecimal digits. */
const DRY_RUN_IDENTIFIER = "0".repeat(32);

/** Check configuration or operate one explicit deployment. */
export async function run(arguments_: string[]): Promise<void> {
    // split the action from its deployment selection
    const [action, ...selection] = arguments_;

    // serve the dev universe
    if (action === "universe") {
        if (selection.length) {
            throw new Error("universe takes no deployment arguments");
        }
        await serveUniverse();

        return;
    }

    // bundle every placed process's Worker and build it with wrangler without deploying
    if (action === "rehearse") {
        const [environment, ...rest] = selection;
        if ((environment !== "development" && environment !== "production") || rest.length) {
            throw new Error("rehearse takes development or production");
        }
        await rehearse(environment);

        return;
    }

    // migrate every placed process's database and deploy its Worker over the applied deployments' outputs
    if (action === "release") {
        const [environment, ...rest] = selection;
        if ((environment !== "development" && environment !== "production") || rest.length) {
            throw new Error("release takes development or production");
        }
        await release(environment);

        return;
    }

    // put a placed process's secrets from the environment, as an operator does once before its first release
    if (action === "secrets") {
        const [environment, name, ...rest] = selection;
        if (
            (environment !== "development" && environment !== "production") ||
            name === undefined ||
            rest.length
        ) {
            throw new Error("secrets takes development or production and a process name");
        }
        await putSecrets(environment, name);

        return;
    }

    // migrate a process's database at the URL the environment names, as a release does before its Worker deploys
    if (action === "migrate") {
        const [name, ...rest] = selection;
        const url = process.env["DATABASE_URL"];
        if (name === undefined || rest.length || url === undefined) {
            throw new Error("migrate takes a process name, with DATABASE_URL set");
        }
        await migrate(name, url);

        return;
    }

    // format the roots, or check their formatting
    if (action === "format" || action === "format-check") {
        if (selection.length) {
            throw new Error(`${action} takes no deployment arguments`);
        }
        execute(
            "tofu",
            [
                "fmt",
                ...(action === "format-check" ? ["-check"] : []),
                "-recursive",
                "src",
                "deployment",
            ],
            { ...process.env },
        );

        return;
    }

    // validate each root without opening remote state
    if (action === "validate") {
        if (selection.length) {
            throw new Error("validate takes no deployment arguments");
        }

        // validate every deployment with its own variables
        for (const checked of [
            ["shared"],
            ["development", "universe"],
            ["development", "eu"],
            ["development", "us"],
            ["production", "universe"],
            ["production", "eu"],
            ["production", "us"],
        ]) {
            const deployment = new Deployment(checked);
            const directory = deployment.directory;
            const environment = {
                ...process.env,
                TF_DATA_DIR: resolve(ROOT, ".terraform", "check", deployment.name),
            };
            execute(
                "tofu",
                [
                    `-chdir=${directory}`,
                    "init",
                    "-backend=false",
                    "-input=false",
                    "-lockfile=readonly",
                    ...deployment.variables,
                ],
                environment,
            );
            execute(
                "tofu",
                [`-chdir=${directory}`, "validate", ...deployment.variables],
                environment,
            );
        }

        return;
    }

    // select deployment-specific state, plans and credentials
    if (action !== "init" && action !== "diff" && action !== "deploy") {
        throw new Error(
            "use format, format-check, validate, universe, rehearse, release, secrets, migrate, init, diff, or deploy",
        );
    }
    const deployment = new Deployment(selection);
    const environment = await initialize(deployment);
    const directory = `-chdir=${deployment.directory}`;

    // apply only the reviewed plan for this deployment
    if (action !== "init") {
        const planDirectory = await mkdtemp(resolve(deployment.cache, "plan-"));
        const plan = resolve(planDirectory, "deployment.tfplan");
        execute(
            "tofu",
            [
                directory,
                "plan",
                "-input=false",
                "-lock-timeout=60s",
                ...deployment.variables,
                `-out=${plan}`,
            ],
            environment,
        );
        if (action === "deploy") {
            // confirm outside CI
            if (process.env["CI"] !== "true" && !confirm(`Apply ${deployment.name}?`)) {
                throw new Error("deployment cancelled");
            }
            execute(
                "tofu",
                [directory, "apply", "-input=false", "-lock-timeout=60s", plan],
                environment,
            );
        }
    }
}

/** Initialize a deployment's root against its remote state, returning the environment its tools run with. */
async function initialize(deployment: Deployment): Promise<NodeJS.ProcessEnv> {
    // run the tools with the resolved credentials and the deployment's own cache
    const resolved = await credentials();
    const environment = {
        ...process.env,
        ...resolved.environment,
        TF_DATA_DIR: deployment.cache,
    };
    await mkdir(deployment.cache, { recursive: true, mode: 0o700 });

    // write the state credentials to a private backend file
    const backend = resolve(deployment.cache, "state.tfbackend");
    await writeFile(
        backend,
        Object.entries(resolved.backend)
            .map(([name, value]) => `${name} = ${JSON.stringify(value)}\n`)
            .join(""),
        { mode: 0o600 },
    );
    execute(
        "tofu",
        [
            `-chdir=${deployment.directory}`,
            "init",
            "-input=false",
            "-reconfigure",
            "-lockfile=readonly",
            ...deployment.variables,
            `-backend-config=key=${deployment.state}`,
            `-backend-config=${backend}`,
        ],
        environment,
    );

    return environment;
}

/** Build every placed process's Worker of an environment over placeholder outputs without deploying. */
async function rehearse(environment: Environment): Promise<void> {
    for (const placed of Placement.spread(PLATFORM).processes) {
        // bundle the script and write its configuration over placeholder outputs
        const script = new WorkerScript(placed, environment);
        const configuration = await writeScript(script, environment, placed.name, {
            account: DRY_RUN_IDENTIFIER,
            processes: { [placed.name]: DRY_RUN_IDENTIFIER },
            ...(placed.residency === undefined
                ? {}
                : { packages: `destack-${environment}-packages-${placed.residency.code}` }),
        });

        // build the upload as wrangler deploys it and name the secrets it reads
        process.stdout.write(`${script.name} reads secrets ${script.secrets.join(", ")}\n`);
        execute(
            resolve(ROOT, "node_modules/.bin/wrangler"),
            [
                "deploy",
                "--dry-run",
                "--outdir",
                resolve(configuration, "..", "dist"),
                "--config",
                configuration,
            ],
            { ...process.env, WRANGLER_SEND_METRICS: "false" },
        );
    }
}

/** The outputs a deployment's root keeps beside its Workers' bindings: each process's database role. */
interface ReleaseOutputs extends DeploymentOutputs {
    /** Each process's database role, by process name. */
    readonly roles: Readonly<Record<string, DatabaseRole>>;
}

/** A process's database role, as the deployment's database module outputs it. */
interface DatabaseRole {
    /** The host the role connects to. */
    readonly host: string;
    /** The role's user. */
    readonly user: string;
    /** The role's password. */
    readonly password: string;
}

/** Migrate and deploy every placed process of an environment, checking each answers. */
async function release(environment: Environment): Promise<void> {
    // read each applied deployment's outputs once
    const outputs = new Map<string, ReleaseOutputs>();
    const read = async (scope: string): Promise<ReleaseOutputs> => {
        // reuse a deployment's outputs read before, or initialize it and read them
        const known = outputs.get(scope);
        if (known !== undefined) {
            return known;
        }
        const deployment = new Deployment([environment, scope]);
        const deployed = readOutputs(deployment, await initialize(deployment));
        outputs.set(scope, deployed);

        return deployed;
    };

    // migrate each process's database before its Worker deploys, as its new code expects
    const wrangler = {
        ...process.env,
        ...(await credentials()).environment,
        WRANGLER_SEND_METRICS: "false",
    };
    const scripts: WorkerScript[] = [];
    for (const placed of Placement.spread(PLATFORM).processes) {
        const deployed = await read(placed.residency?.code ?? "universe");
        const role = deployed.roles[placed.name];
        if (role === undefined) {
            throw new Error(`no database role of ${placed.name} in the ${environment} outputs`);
        }
        await migrate(placed.name, roleUrl(role));

        // deploy the Worker over the deployment's outputs
        const script = new WorkerScript(placed, environment);
        const configuration = await writeScript(script, environment, placed.name, {
            account: deployed.account,
            processes: deployed.processes,
            ...(deployed.packages === undefined ? {} : { packages: deployed.packages }),
        });
        execute(
            resolve(ROOT, "node_modules/.bin/wrangler"),
            ["deploy", "--config", configuration],
            wrangler,
        );
        scripts.push(script);
    }

    // check every deployed origin answers without a server failure
    for (const script of scripts) {
        await smoke(script);
    }
}

/** Put a process's secrets from the environment through wrangler, refusing any the environment lacks. */
async function putSecrets(environment: Environment, name: string): Promise<void> {
    // find the process and require each secret it reads
    const placed = Placement.spread(PLATFORM).processes.find((entry) => entry.name === name);
    if (placed === undefined) {
        throw new Error(`no process ${name}`);
    }
    const script = new WorkerScript(placed, environment);
    const missing = script.secrets.filter((secret) => process.env[secret] === undefined);
    if (missing.length) {
        throw new Error(`${name} reads secrets the environment lacks: ${missing.join(", ")}`);
    }

    // write them to a private file and put them in one bulk upload
    const directory = await mkdtemp(resolve(tmpdir(), "destack-secrets-"));
    try {
        const file = resolve(directory, "secrets.json");
        const values = Object.fromEntries(
            script.secrets.map((secret) => [secret, process.env[secret]]),
        );
        await writeFile(file, JSON.stringify(values), { mode: 0o600 });
        execute(
            resolve(ROOT, "node_modules/.bin/wrangler"),
            ["secret", "bulk", file, "--name", script.name],
            {
                ...process.env,
                ...(await credentials()).environment,
                WRANGLER_SEND_METRICS: "false",
            },
        );
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
}

/** Read a deployment's outputs: its account, processes' Hyperdrive configurations, package bucket and database roles. */
function readOutputs(deployment: Deployment, environment: NodeJS.ProcessEnv): ReleaseOutputs {
    // read every output as JSON, sensitive ones included
    const command = spawnSync("tofu", [`-chdir=${deployment.directory}`, "output", "-json"], {
        cwd: ROOT,
        env: environment,
        encoding: "utf8",
    });
    if (command.status !== 0) {
        throw new Error(`tofu output of ${deployment.name} failed: ${command.stderr}`);
    }
    const parsed = OutputsSchema.parse(JSON.parse(command.stdout));

    return {
        account: parsed.account.value,
        processes: parsed.processes.value,
        roles: parsed.roles.value,
        ...(parsed.packages === undefined ? {} : { packages: parsed.packages.value }),
    };
}

/** The outputs `tofu output -json` prints for a universe or residency root. */
const OutputsSchema = schema.object({
    account: schema.object({ value: schema.string().min(1) }),
    processes: schema.object({ value: schema.record(schema.string(), schema.string().min(1)) }),
    packages: schema.object({ value: schema.string().min(1) }).exactOptional(),
    roles: schema.object({
        value: schema.record(
            schema.string(),
            schema.object({
                host: schema.string().min(1),
                user: schema.string().min(1),
                password: schema.string().min(1),
            }),
        ),
    }),
});

/** Write a URL connecting to a process's database as its role. */
function roleUrl(role: DatabaseRole): string {
    // connect over verified TLS to the branch's default database
    const url = new URL(`postgresql://${role.host}/postgres`);
    url.username = role.user;
    url.password = role.password;
    url.searchParams.set("sslmode", "verify-full");

    return url.href;
}

/** Bundle a process's Worker and write its wrangler configuration over a deployment's outputs, returning the configuration's path. */
async function writeScript(
    script: WorkerScript,
    environment: Environment,
    name: string,
    outputs: DeploymentOutputs,
): Promise<string> {
    // bundle the script into a fresh directory with its configuration beside it
    const directory = resolve(ROOT, ".wrangler", environment, name);
    await rm(directory, { recursive: true, force: true });
    await script.build(directory);
    const configuration = resolve(directory, "wrangler.json");
    await writeFile(configuration, `${JSON.stringify(script.configuration(outputs), null, 4)}\n`);

    return configuration;
}

/** Require a deployed origin to answer without a server failure, the account its OpenID configuration. */
async function smoke(script: WorkerScript): Promise<void> {
    // request the issuer's discovery document from the account, and the origin from other processes
    const isAccount = script.origin === script.issuer;
    const url = isAccount
        ? new URL("/.well-known/openid-configuration", script.issuer)
        : new URL("/", script.origin);
    const response = await fetch(url, { redirect: "manual" });
    if (response.status >= 500 || (isAccount && !response.ok)) {
        throw new Error(`${script.name} answered ${url.href} with ${response.status}`);
    }
}

/** Create a process's schema for its role and migrate its database there. */
async function migrate(name: string, url: string): Promise<void> {
    // find the process's database
    const placed = Placement.spread(PLATFORM).processes.find((entry) => entry.name === name);
    const declaration = placed?.packages[0]?.database;
    if (declaration === undefined) {
        throw new Error(`no process ${name}`);
    }

    // migrate the schema named after the connecting role
    const database = await postgresql.connect(url, declaration);
    try {
        await database.execute(sql.raw("CREATE SCHEMA IF NOT EXISTS AUTHORIZATION CURRENT_USER"));
        await database.migrate(declaration.tables);
    } finally {
        await database.close();
    }
}

/** Execute a tool in the stack root with inherited output, preserving its failures. */
function execute(tool: string, arguments_: string[], environment: NodeJS.ProcessEnv): void {
    // run the tool and refuse any failed run
    const command = spawnSync(tool, arguments_, {
        cwd: ROOT,
        stdio: "inherit",
        env: environment,
    });
    if (command.error !== undefined) {
        throw command.error;
    }
    if (command.signal !== null) {
        throw new Error(`${tool} terminated by ${command.signal}`);
    }
    if (command.status !== 0) {
        throw new Error(`${tool} exited with ${command.status}`);
    }
}

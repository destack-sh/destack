import { resolve } from "node:path";
import type { Environment, HostCapability } from "@destack/environment";
import { CloudflareCredentials } from "./cloudflare/credential.ts";
import type { EnvelopeConfiguration } from "@destack/identity/aws";
import type { S3Location } from "@destack/bucket/s3";
import { schema } from "@destack/schema";
import { RESIDENCIES } from "@platform/stack";

/** The release package's directory. */
export const ROOT = resolve(import.meta.dirname, "../..");

/** The variables of this process every tool runs with: its user, paths, language and CI. */
const HOST_VARIABLES: ReadonlySet<string> = new Set([
    "HOME",
    "PATH",
    "TMPDIR",
    "LANG",
    "LC_ALL",
    "USER",
    "CI",
]);

/** The prefixes of the variables OpenTofu and its AWS provider read. */
export const TOFU_VARIABLES = ["TF_", "AWS_"] as const;

/** The prefixes of the variables wrangler reads beside the deployment's credentials. */
export const WRANGLER_VARIABLES = ["WRANGLER_", "CLOUDFLARE_ACCOUNT_ID"] as const;

/** A universe or residency root's outputs for its Worker, as `tofu output -json` prints them. */
const WorkerOutputs = schema.object({
    kms_region: output(schema.string().min(1)),
    kms_key_arn: output(schema.string().min(1)),
    access_key_id: output(schema.string().min(1)),
    secret_access_key: output(schema.string().min(1)),
});

/** The outputs of a universe root, as `tofu output -json` prints them. */
const UniverseOutputs = WorkerOutputs.extend({
    account: output(schema.string().min(1)),
    hyperdrive: output(schema.string().min(1)),
    files: output(schema.string().min(1)),
    files_region: output(schema.string().min(1)),
});

/** The outputs of a residency root, as `tofu output -json` prints them. */
const ResidencyOutputs = WorkerOutputs.extend({
    account: output(schema.string().min(1)),
    files: output(schema.string().min(1)),
    packages: output(schema.string().min(1)),
    host: output(schema.string().min(1)),
    hyperdrive: output(schema.string().min(1)),
});

/** An access key and its secret, as AWS and R2's S3 API sign with them. */
export interface AccessKey {
    /** The access key's identifier. */
    readonly accessKeyId: string;
    /** The secret it signs with. */
    readonly secretAccessKey: string;
}

/** What a root provisions for its Worker: its root keys' KMS key and its one AWS access key. */
export interface WorkerOutputs {
    /** The envelope of the KMS key sealing the Worker's root keys. */
    readonly envelope: EnvelopeConfiguration;
    /** The Worker's access key: decrypting its own keys, sending mail, keeping its bucket. */
    readonly access: AccessKey;
}

/** What an applied universe root provisions. */
export interface UniverseOutputs extends WorkerOutputs {
    /** The Cloudflare account running the universe Worker. */
    readonly account: string;
    /** The Hyperdrive configuration the universe Worker connects through. */
    readonly hyperdrive: string;
    /** The S3 bucket keeping the universe's files: its call events' segments and its blobs. */
    readonly files: S3Location;
}

/** What an applied residency root provisions. */
export interface ResidencyOutputs extends WorkerOutputs {
    /** The Cloudflare account running the machine Worker. */
    readonly account: string;
    /** The residency's files bucket. */
    readonly files: string;
    /** The residency's package bucket, keeping its cloud machine's build store. */
    readonly packages: string;
    /** The cloud machine's host, which its Worker's routes answer. */
    readonly host: string;
    /** The Hyperdrive configuration the machine Worker reaches the residency's database through. */
    readonly hyperdrive: string;
}

/** The roots no stage selects: the platform's identities and its global infrastructure. */
export const STAGELESS_ROOTS = ["access", "global"] as const;

/** An action over one deployment's state, plans and credentials, OpenTofu's verbs. */
export type RootAction = "init" | "diff" | "deploy";

/** An OpenTofu root: infrastructure managed apart from the others. */
export class Root {
    /** The deployment path within its stage. */
    readonly name: string;
    /** The OpenTofu root directory. */
    readonly directory: string;
    /** The variable arguments: the deployment's variable file. */
    readonly variables: readonly string[];
    /** The remote state object key. */
    readonly state: string;
    /** The isolated OpenTofu working directory. */
    readonly cache: string;
    /** The host the tools run on. */
    readonly environment: Environment<HostCapability>;

    /** Select the platform's identities, the global infrastructure, or one stage and scope. */
    constructor(selection: readonly string[], environment: Environment<HostCapability>) {
        // select a stageless root, or one stage and scope
        const [stage, scope] = selection;
        if (selection.length === 1 && Root.isStageless(stage)) {
            this.name = stage;
            this.directory = resolve(ROOT, "src/deployment/roots", stage);
            this.state = `${stage}.tfstate`;
        }
        // select one stage and scope
        else if (
            selection.length === 2 &&
            stage === "production" &&
            (scope === "universe" || RESIDENCIES.some((residency) => residency.code === scope))
        ) {
            this.name = `${stage}/${scope}`;
            this.directory = resolve(
                ROOT,
                "src/deployment/roots",
                scope === "universe" ? "universe" : "residency",
            );
            this.state = `${stage}/${scope}.tfstate`;
        }
        // refuse anything else
        else {
            const scopes = ["universe", ...RESIDENCIES.map((residency) => residency.code)];
            throw new Error(
                `select '${STAGELESS_ROOTS.join(", ")}', or production followed by '${scopes.join("|")}'`,
            );
        }

        // derive paths after validating the deployment selection
        this.variables = [`-var-file=${resolve(ROOT, "deployment", `${this.name}.tfvars`)}`];
        this.cache = resolve(ROOT, ".terraform", this.name);
        this.environment = environment;
    }

    /** Report whether a name selects a root no stage selects. */
    static isStageless(name: string | undefined): name is (typeof STAGELESS_ROOTS)[number] {
        return STAGELESS_ROOTS.some((stageless) => stageless === name);
    }

    /** List a stage's roots in dependency order: global, the universe, then each residency. */
    static all(stage: string, environment: Environment<HostCapability>): Root[] {
        const scopes = ["universe", ...RESIDENCIES.map((residency) => residency.code)];

        return [["global"], ...scopes.map((scope) => [stage, scope])].map(
            (selection) => new Root(selection, environment),
        );
    }

    /** Validate the access root and each production root, without opening remote state. */
    static async validate(environment: Environment<HostCapability>): Promise<void> {
        const access = new Root(["access"], environment);
        for (const deployment of [access, ...Root.all("production", environment)]) {
            const directory = `-chdir=${deployment.directory}`;
            const variables = {
                ...Root.inherited(environment.process.variables, TOFU_VARIABLES),
                TF_DATA_DIR: resolve(ROOT, ".terraform", "check", deployment.name),
            };
            const initialization = ["init", "-backend=false", "-input=false", "-lockfile=readonly"];
            await Root.execute(
                "tofu",
                [directory, ...initialization, ...deployment.variables],
                variables,
                environment,
            );
            await Root.execute(
                "tofu",
                [directory, "validate", ...deployment.variables],
                variables,
                environment,
            );
        }
    }

    /** Format every root, or check their formatting. */
    static async format(isChecking: boolean, environment: Environment<"process">): Promise<void> {
        const check = isChecking ? ["-check"] : [];
        await Root.execute(
            "tofu",
            ["fmt", ...check, "-recursive", "src", "deployment"],
            Root.inherited(environment.process.variables, TOFU_VARIABLES),
            environment,
        );
    }

    /** Read the variables a tool runs with: the host's and those of its prefixes. */
    static inherited(
        variables: Readonly<Record<string, string | undefined>>,
        prefixes: readonly string[],
    ): Record<string, string> {
        // leave out every other variable, such as the root key and the company's tokens
        return Object.fromEntries(
            Object.entries(variables).flatMap(([name, value]) =>
                value !== undefined &&
                (HOST_VARIABLES.has(name) || prefixes.some((prefix) => name.startsWith(prefix)))
                    ? [[name, value]]
                    : [],
            ),
        );
    }

    /** Execute a tool in the stack root with inherited output, refusing a failed run. */
    static async execute(
        tool: string,
        arguments_: readonly string[],
        variables: Readonly<Record<string, string>>,
        environment: Environment<"process">,
    ): Promise<void> {
        // run the tool with the terminal's streams
        const command = environment.process.spawn([tool, ...arguments_], {
            workingDirectory: ROOT,
            variables,
            stdin: "inherit",
            stdout: "inherit",
            stderr: "inherit",
        });
        const status = await command.exited;

        // refuse a run a signal ended or that failed
        if (command.signal !== undefined) {
            throw new Error(`'${tool}' terminated by '${command.signal}'`);
        } else if (status !== 0) {
            throw new Error(`'${tool}' exited with ${status}`);
        }
    }

    /** Read an environment variable the operator sets, refusing an absent or empty one. */
    static required(name: string, environment: Environment<"process">): string {
        const value = environment.process.variables[name];
        if (value === undefined || value === "") {
            throw new Error(`'${name}' is required`);
        }

        return value;
    }

    /** Read an applied universe root's outputs. */
    async universe(variables: Readonly<Record<string, string>>): Promise<UniverseOutputs> {
        const parsed = UniverseOutputs.parse(await this.#outputs(variables));
        const region = parsed.files_region.value;

        return {
            account: parsed.account.value,
            hyperdrive: parsed.hyperdrive.value,
            files: {
                endpoint: new URL(`https://s3.${region}.amazonaws.com`),
                bucket: parsed.files.value,
                region,
            },
            ...workerOf(parsed),
        };
    }

    /** Read an applied residency root's outputs. */
    async residency(variables: Readonly<Record<string, string>>): Promise<ResidencyOutputs> {
        const parsed = ResidencyOutputs.parse(await this.#outputs(variables));

        return {
            account: parsed.account.value,
            files: parsed.files.value,
            packages: parsed.packages.value,
            host: parsed.host.value,
            hyperdrive: parsed.hyperdrive.value,
            ...workerOf(parsed),
        };
    }

    /** Initialize the deployment, then plan it, or apply the reviewed plan. */
    async operate(action: RootAction): Promise<void> {
        // select deployment-specific state, plans and credentials
        const { environment } = this;
        const variables = await this.initialize();
        const directory = `-chdir=${this.directory}`;

        // plan, and apply only the reviewed plan for this deployment
        if (action !== "init") {
            const planDirectory = await environment.files.makeTemporaryDirectory("plan-", {
                directory: this.cache,
            });
            const plan = resolve(planDirectory, "deployment.tfplan");
            const options = ["-input=false", "-lock-timeout=60s"];
            try {
                await Root.execute(
                    "tofu",
                    [directory, "plan", ...options, ...this.variables, `-out=${plan}`],
                    variables,
                    environment,
                );

                // confirm outside CI, and apply the plan
                if (action === "deploy") {
                    const isConfirmed =
                        environment.process.variables["CI"] === "true" ||
                        confirm(`Apply ${this.name}?`);
                    if (!isConfirmed) {
                        throw new Error("deployment cancelled");
                    }
                    await Root.execute(
                        "tofu",
                        [directory, "apply", ...options, plan],
                        variables,
                        environment,
                    );
                }
            } finally {
                // remove the plan, which has the deployment's sensitive values
                await environment.files.remove(planDirectory, { recursive: true, force: true });
            }
        }
    }

    /** Initialize the root against its remote state, answering its tools' variables. */
    async initialize(): Promise<Readonly<Record<string, string>>> {
        // run the tools with the resolved credentials and the deployment's own cache
        const { environment } = this;
        const resolved = await CloudflareCredentials.resolve(environment);
        const variables = {
            ...Root.inherited(environment.process.variables, TOFU_VARIABLES),
            ...resolved.environment,
            TF_DATA_DIR: this.cache,
        };
        await environment.files.mkdir(this.cache, { recursive: true, mode: 0o700 });

        // write the state credentials to a fresh private backend file, its mode set at creation
        const backend = resolve(this.cache, "state.tfbackend");
        await environment.files.remove(backend, { force: true });
        await environment.files.writeTextFile(
            backend,
            Object.entries(resolved.backend)
                .map(([name, value]) => `${name} = ${JSON.stringify(value)}\n`)
                .join(""),
            { mode: 0o600 },
        );
        await Root.execute(
            "tofu",
            [
                `-chdir=${this.directory}`,
                "init",
                "-input=false",
                "-reconfigure",
                "-lockfile=readonly",
                ...this.variables,
                `-backend-config=key=${this.state}`,
                `-backend-config=${backend}`,
            ],
            variables,
            environment,
        );

        return variables;
    }

    /** Read every output of the applied root as JSON, sensitive ones included. */
    async #outputs(variables: Readonly<Record<string, string>>): Promise<unknown> {
        // run the tool, keeping its output and diagnostics
        const command = this.environment.process.spawn(
            ["tofu", `-chdir=${this.directory}`, "output", "-json"],
            { workingDirectory: ROOT, variables, stdout: "pipe", stderr: "pipe" },
        );
        const ran = await command.output();
        const decoder = new TextDecoder();

        // refuse a failed run
        if (ran.code !== 0) {
            throw new Error(
                `tofu output of '${this.name}' failed: '${decoder.decode(ran.stderr)}'`,
            );
        }

        return JSON.parse(decoder.decode(ran.stdout));
    }
}

/** Read what a root provisions for its Worker: its key's envelope and its access key. */
function workerOf(parsed: schema.Infer<typeof WorkerOutputs>): WorkerOutputs {
    return {
        envelope: {
            kind: "aws-kms",
            region: parsed.kms_region.value,
            keyId: parsed.kms_key_arn.value,
        },
        access: {
            accessKeyId: parsed.access_key_id.value,
            secretAccessKey: parsed.secret_access_key.value,
        },
    };
}

/** Describe one output as `tofu output -json` prints it: its sensitivity, type and value. */
function output<Value extends schema.Schema>(value: Value) {
    return schema.object({ sensitive: schema.boolean(), type: schema.unknown(), value });
}

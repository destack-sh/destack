import { resolve } from "node:path";
import { DOMAINS } from "@destack/host";
import { modulePlugin } from "@destack/package/bun";
import { ServiceMount } from "@destack/service";
import type { Workload } from "@destack/service/workload";
import { ROOT } from "../../deployment/index.ts";
import type { Process } from "../../placement/index.ts";

/** The workerd release the scripts run on, as the release Worker pins it. */
const COMPATIBILITY_DATE = "2026-07-30";

/** The package conditions selecting each dependency's workerd build, such as the PostgreSQL driver over Cloudflare sockets. */
const CONDITIONS = ["workerd", "worker"];

/** The SES region sending sign-in mail, the shared root's mail region. */
const MAIL_REGION = "eu-central-1";

/** The sender of sign-in mail. */
const MAIL_FROM = "Destack <sign-in@destack.app>";

/** The Durable Object location of universe processes near their eu-central database. */
const UNIVERSE_LOCATION = { jurisdiction: "", location: "weur" } as const;

/** The environments a universe deploys. */
export type Environment = "development" | "production";

/** The outputs of the OpenTofu deployment a script binds, as `tofu output` reads them. */
export interface DeploymentOutputs {
    /** The Cloudflare account running the deployment. */
    readonly account: string;
    /** The Hyperdrive configuration of each process, by process name. */
    readonly processes: Readonly<Record<string, string>>;
    /** The residency's package bucket, absent for the universe deployment. */
    readonly packages?: string;
}

/** A placed process's Worker script with its entry, Durable Object and bindings. */
export class WorkerScript {
    /** The process the script runs. */
    readonly process: Process;
    /** The environment the script deploys to. */
    readonly environment: Environment;
    /** The one workload the process runs. */
    readonly workload: Workload;

    /** Describe the script of a process running one workload, refusing a process running several. */
    constructor(process: Process, environment: Environment) {
        // require one workload
        const [placed, ...others] = process.packages;
        if (placed === undefined || others.length > 0) {
            throw new TypeError(
                `a Worker runs one workload, ${process.name} runs ${process.packages.length}`,
            );
        }
        this.process = process;
        this.environment = environment;
        this.workload = placed.workload;
    }

    /** The script's name, such as `destack-production-forge-eu`. */
    get name(): string {
        return WorkerScript.#scriptName(this.environment, this.process.name);
    }

    /** The module running the workload's Durable Object. */
    get entry(): string {
        return resolve(ROOT, "src/cloudflare/worker", `${this.workload.name}.ts`);
    }

    /** The class of the Durable Object running the workload, such as `DurableForge`. */
    get className(): string {
        const name = this.workload.name;

        return `Durable${name.charAt(0).toUpperCase()}${name.slice(1)}`;
    }

    /** The secrets an operator puts before the script's first deploy, by binding name. */
    get secrets(): readonly string[] {
        // read the secrets of every process, every placed process and every residency process
        const shared = ["DESTACK_CALL_KEY"];
        const placed = ["DESTACK_PLACEMENT_ID", "DESTACK_HOST_ID", "DESTACK_HOST_KEY"];
        const residency = [...placed, "DESTACK_REGION_ID"];
        switch (this.workload.name) {
            case "account":
                return [
                    ...shared,
                    "DESTACK_SECRET",
                    "DESTACK_OPERATOR",
                    ...(this.isSendingMail ? ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"] : []),
                ];
            case "forge":
                return [...shared, ...residency, "DESTACK_ARTIFACTS_TOKEN"];
            case "relay":
                return [...shared, ...placed];
            default:
                throw new TypeError(`no Worker runs ${this.workload.name}`);
        }
    }

    /** Whether the account sends sign-in mail through SES, which production does and development prints instead. */
    get isSendingMail(): boolean {
        return this.environment === "production";
    }

    /** The account service's origin: the universe's issuer. */
    get issuer(): string {
        return this.environment === "production"
            ? "https://destack.app"
            : "https://development.destack.app";
    }

    /** The origin the process answers at. */
    get origin(): string {
        // select the relay's origin, the issuer's or the region's
        const suffix = this.environment === "production" ? "" : "-development";
        const residency = this.process.residency;
        if (this.workload.name === "relay") {
            return `https://relay${suffix}.destack.space`;
        } else if (residency === undefined) {
            return this.issuer;
        }

        return `https://${residency.code}${suffix}.destack.cloud`;
    }

    /** The proxied DNS names of the script's zone routes, which OpenTofu keeps. */
    get names(): readonly string[] {
        switch (this.workload.name) {
            case "forge":
                return [new URL(this.origin).host];
            case "relay":
                return this.environment === "production"
                    ? Object.values(DOMAINS).map((domain) => `*.${domain}`)
                    : [];
            default:
                return [];
        }
    }

    /** Write the wrangler configuration of the script, binding the deployment's outputs. */
    configuration(outputs: DeploymentOutputs): WranglerConfiguration {
        // require the process's Hyperdrive configuration
        const hyperdrive = outputs.processes[this.process.name];
        if (hyperdrive === undefined) {
            throw new Error(
                `the deployment provisions no Hyperdrive configuration for ${this.process.name}`,
            );
        }

        // keep the Durable Object in the residency's jurisdiction, near its database
        const residency = this.process.residency;
        const { jurisdiction, location } = residency === undefined ? UNIVERSE_LOCATION : residency;
        const host = new URL(this.origin).host;
        const shared = {
            name: this.name,
            main: "index.js",
            no_bundle: true,
            compatibility_date: COMPATIBILITY_DATE,
            compatibility_flags: ["nodejs_compat"],
            workers_dev: false,
            preview_urls: false,
            observability: { enabled: true },
            durable_objects: { bindings: [{ name: "WORKLOAD", class_name: this.className }] },
            migrations: [{ tag: "v1", new_sqlite_classes: [this.className] }],
            hyperdrive: [{ binding: "DATABASE", id: hyperdrive }],
        };
        const vars = {
            DESTACK_PROCESS: this.process.name,
            DESTACK_JURISDICTION: jurisdiction,
            DESTACK_LOCATION: location,
            DESTACK_ISSUER: this.issuer,
        };

        // reach the account process directly, and answer below the workload's mount at the region's origin
        const account = {
            binding: "ACCOUNT",
            service: WorkerScript.#scriptName(this.environment, "account"),
        };
        const mount = {
            pattern: `${host}${ServiceMount.path(this.workload.package.id)}*`,
            zone_name: "destack.cloud",
        };
        switch (this.workload.name) {
            // answer for the whole issuer's host with the account service
            case "account":
                return {
                    ...shared,
                    routes: [{ pattern: host, custom_domain: true }],
                    vars: {
                        ...vars,
                        DESTACK_MAIL_REGION: this.isSendingMail ? MAIL_REGION : "",
                        DESTACK_MAIL_FROM: this.isSendingMail ? MAIL_FROM : "",
                    },
                };

            // answer below the forge's mount at the region's origin, keeping builds in the residency's bucket and repositories in Artifacts
            case "forge":
                return {
                    ...shared,
                    routes: [mount],
                    services: [account],
                    vars: {
                        ...vars,
                        DESTACK_ORIGIN: this.origin,
                        DESTACK_ARTIFACTS_ACCOUNT: outputs.account,
                        DESTACK_ARTIFACTS_NAMESPACE: `destack-${this.environment}-${this.#residency().code}`,
                    },
                    r2_buckets: [
                        {
                            binding: "PACKAGES",
                            bucket_name: this.#packages(outputs),
                            ...(jurisdiction === "" ? {} : { jurisdiction }),
                        },
                    ],
                };

            // take tunnels at the relay's origin, and in production every name of the space and device domains
            case "relay":
                return {
                    ...shared,
                    routes: [
                        { pattern: host, custom_domain: true },
                        ...(this.environment === "production"
                            ? [DOMAINS.space, DOMAINS.host].map((domain) => ({
                                  pattern: `*.${domain}/*`,
                                  zone_name: domain,
                              }))
                            : []),
                    ],
                    services: [account],
                    vars: { ...vars, DESTACK_ORIGIN: this.origin },
                };
            default:
                throw new TypeError(`no Worker runs ${this.workload.name}`);
        }
    }

    /** Read the residency the process serves, refusing a process serving every residency. */
    #residency(): NonNullable<Process["residency"]> {
        const residency = this.process.residency;
        if (residency === undefined) {
            throw new TypeError(`${this.process.name} serves no residency`);
        }

        return residency;
    }

    /** Read the residency's package bucket the deployment provisions. */
    #packages(outputs: DeploymentOutputs): string {
        if (outputs.packages === undefined) {
            throw new Error(`the deployment provisions no package bucket for ${this.process.name}`);
        }

        return outputs.packages;
    }

    /** Bundle the script's entry for workerd into a directory, refusing a bundle that imports Bun. */
    async build(directory: string): Promise<string> {
        // bundle with module metadata, leaving workerd's own modules to the runtime
        const output = await Bun.build({
            entrypoints: [this.entry],
            outdir: directory,
            naming: "index.js",
            target: "browser",
            format: "esm",
            minify: true,
            conditions: CONDITIONS,
            external: ["cloudflare:*", "node:*", "bun", "bun:*"],
            plugins: [modulePlugin],
        });
        const [bundle] = output.outputs;
        if (bundle === undefined) {
            throw new Error(`${this.name} bundled no module`);
        }

        // refuse a bundle still importing a Bun module, which workerd lacks
        const code = await bundle.text();
        const imported = /\bfrom\s*"(bun(?::[a-z]+)?)"/u.exec(code)?.[1];
        if (imported !== undefined) {
            throw new Error(`${this.name} imports ${imported}, which workerd lacks`);
        }

        return bundle.path;
    }

    /** Name the script of a process in an environment. */
    static #scriptName(environment: Environment, process: string): string {
        return `destack-${environment}-${process}`;
    }
}

/** A Worker's wrangler configuration, as `wrangler.json` takes it. */
export type WranglerConfiguration = Readonly<Record<string, unknown>>;

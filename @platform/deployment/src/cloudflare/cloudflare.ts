import { resolve } from "node:path";
import { CHECKOUT, PlatformBuild, releaseVersion } from "@platform/distribution";
import { type PlatformRegion, REGIONS } from "@platform/stack";
import { present } from "@destack/schema";
import { Custody, type CustodyEnvelopes } from "../deployment/custody.ts";
import { Deployment } from "../deployment/deployment.ts";
import { Root, type RegionOutputs, type UniverseOutputs } from "../deployment/root.ts";
import { WorkerScript } from "./script.ts";

/** A deployment on Cloudflare: its roots, the custody record of its keys, and its Workers. */
export class Cloudflare {
    /** The deployment. */
    readonly deployment: Deployment;

    /** Run a deployment on Cloudflare. */
    constructor(deployment: Deployment) {
        this.deployment = deployment;
    }

    /** The directory of the asset layers, one directory per layer under its digest. */
    get assets(): string {
        return resolve(CHECKOUT, "target", this.deployment.name, "cloud");
    }

    /** Apply the roots in dependency order, then release the Workers onto them. */
    async deploy(): Promise<void> {
        // apply every root in dependency order
        const { name, environment } = this.deployment;
        for (const root of Root.all(name, environment)) {
            await root.operate("deploy");
        }

        // release the Workers onto the applied roots
        await this.release();
    }

    /** Deploy the universe's and every managed machine's Workers onto the applied roots. */
    async release(): Promise<void> {
        // read the operator, the roots' outputs and the custody record the operator sealed
        const { deployment } = this;
        const { environment } = deployment;
        const operator = Root.required("DESTACK_OPERATOR", environment);
        const { outputs, outputsOf } = await this.#outputs();
        const custody = await Custody.read(
            Custody.file(deployment.name),
            Cloudflare.#envelopes(outputs, outputsOf),
            environment,
        );

        // declare the universe over the record with the checkout's releases
        await using build = await PlatformBuild.build(CHECKOUT, environment, {
            version: releaseVersion(environment),
            inputs: "pinned",
        });
        const definition = deployment.definition(operator, custody, build);

        // write the releases the Workers ship as their static assets
        const releases = await build.writeAssets(this.assets);
        deployment.scope.log.info("platform.assets.written", {
            "platform.release.count": build.readers.length,
        });

        // deploy every script in order, the universe first and each machine before its tail
        const scripts = WorkerScript.all(deployment, outputs, outputsOf, releases);
        const modules = WorkerScript.modules(build, definition);
        const providers = deployment.providers((id) =>
            Root.required(Deployment.providerSecret(id), environment),
        );
        for (const [script, configuration] of scripts) {
            const { target } = script;
            const access =
                target.kind === "universe" ? outputs.access : outputsOf(target.region).access;
            const secrets = script.secrets(custody, access, providers);
            await script.deploy(configuration, secrets, modules);
        }

        // check every deployed origin answers once it started, a tail answering none
        for (const [script] of scripts) {
            if (script.target.kind !== "tail") {
                const started = await script.waitReady();
                deployment.scope.log.info("platform.deployment.answered", {
                    "platform.script.name": script.name,
                    "platform.deployment.duration_ms": started,
                });
            }
        }
    }

    /** Seal the keys the custody record lacks under each root's KMS key. */
    async seal(print: (name: string, line: string) => void): Promise<void> {
        // seal as the operator, with the AWS key allowed to generate and encrypt under the roots' keys
        const { name, environment, scope } = this.deployment;
        const credentials = {
            accessKeyId: Root.required("AWS_ACCESS_KEY_ID", environment),
            secretAccessKey: Root.required("AWS_SECRET_ACCESS_KEY", environment),
            ...sessionToken(environment.process.variables["AWS_SESSION_TOKEN"]),
        };

        // seal the keys the record lacks under the applied roots' envelopes
        const { outputs, outputsOf } = await this.#outputs();
        const file = Custody.file(name);
        await Custody.seal(
            file,
            Cloudflare.#envelopes(outputs, outputsOf),
            credentials,
            environment,
        );
        scope.log.info("platform.custody.sealed", {
            "file.path": file,
        });
        print("custody", `commit ${file} before the next deploy`);
    }

    /** Read the applied universe and region roots' outputs. */
    async #outputs(): Promise<{
        readonly outputs: UniverseOutputs;
        readonly outputsOf: (region: PlatformRegion) => RegionOutputs;
    }> {
        // read the universe root's outputs
        const { name, environment } = this.deployment;
        const universe = new Root([name, "universe"], environment);
        const outputs = await universe.universe(await universe.initialize());

        // read each region root's outputs
        const regions = new Map<string, RegionOutputs>();
        for (const region of REGIONS) {
            const root = new Root([name, region.code], environment);
            const variables = await root.initialize();
            regions.set(region.code, await root.region(variables));
        }
        const outputsOf = (region: PlatformRegion) =>
            present(regions.get(region.code), `the outputs of ${region.code}`);

        return { outputs, outputsOf };
    }

    /** List the KMS envelopes the custody record seals under: the universe's and each machine's. */
    static #envelopes(
        outputs: UniverseOutputs,
        outputsOf: (region: PlatformRegion) => RegionOutputs,
    ): CustodyEnvelopes {
        return {
            universe: outputs.envelope,
            machines: new Map(
                REGIONS.map((region) => [region.machine.id, outputsOf(region).envelope]),
            ),
        };
    }
}

/** Read an AWS session token of temporary credentials, none when unset. */
function sessionToken(token: string | undefined): { readonly sessionToken?: string } {
    return token === undefined || token === "" ? {} : { sessionToken: token };
}

import { ArtifactsStorage } from "@destack/forge/cloudflare";
import { forgeDatabase } from "@destack/forge/stack";
import { forgeConfiguration, forgeWorkload } from "@destack/forge/workload";
import { R2Bucket } from "@destack/bucket/cloudflare";
import { PackageStore } from "@destack/build/store";
import { ResourceContext } from "@destack/resource/context";
import { ServiceMount } from "@destack/service";
import { type DurableState, DurableWorkload } from "@destack/service/cloudflare";
import { PlacedProcess, type ResidencyEnvironment } from "./placed.ts";
import { WorkerProcess } from "./process.ts";

/** A forge process's bindings: the residency's package bucket and the Cloudflare Artifacts namespace keeping its platform repositories. */
export interface ForgeEnvironment extends ResidencyEnvironment {
    /** The R2 bucket keeping the residency's builds. */
    readonly PACKAGES: ConstructorParameters<typeof R2Bucket>[0];
    /** The Cloudflare account keeping the repositories. */
    readonly DESTACK_ARTIFACTS_ACCOUNT: string;
    /** The Artifacts namespace keeping the residency's repositories. */
    readonly DESTACK_ARTIFACTS_NAMESPACE: string;
    /** The API token with Artifacts edit permission. */
    readonly DESTACK_ARTIFACTS_TOKEN: string;
}

/** A forge process's Durable Object, which runs the residency's forge. */
export class DurableForge extends DurableWorkload {
    /** Start the forge before the object takes any event. */
    constructor(state: DurableState, environment: ForgeEnvironment) {
        super(state, (alarm) => {
            // keep builds in the residency's bucket and platform repositories in its Artifacts namespace
            const configuration = new ResourceContext().bind(forgeConfiguration, {
                store: new PackageStore(new R2Bucket(environment.PACKAGES)),
                endpoint: new URL(
                    ServiceMount.url(environment.DESTACK_ORIGIN, forgeWorkload.package.id),
                ),
                storage: new ArtifactsStorage({
                    account: environment.DESTACK_ARTIFACTS_ACCOUNT,
                    namespace: environment.DESTACK_ARTIFACTS_NAMESPACE,
                    token: environment.DESTACK_ARTIFACTS_TOKEN,
                }),
            });

            return PlacedProcess.start(
                environment,
                { workload: forgeWorkload, database: forgeDatabase },
                configuration,
                alarm,
            );
        });
    }
}

/** A forge process's Worker, handing every request to its Durable Object. */
export default {
    /** Forward a request to the process's Durable Object. */
    fetch(request: Request, environment: ForgeEnvironment): Promise<Response> {
        return WorkerProcess.forward(environment, request);
    },
};

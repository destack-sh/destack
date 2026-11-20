import type { Environment, HostCapability } from "@destack/environment";
import { IssuerCredential, Session } from "@destack/account/client";
import { Bookmark } from "@destack/object/client";
import { LATEST_TAG } from "@destack/registry/object";
import { CHECKOUT, PlatformBuild, printStatus } from "@platform/distribution";
import { Cloudflare } from "../cloudflare/cloudflare.ts";
import { ActionsToken } from "../github/oidc.ts";
import { Deployment } from "./deployment.ts";
import { Root } from "./root.ts";

/** Validate every deployment's root. */
export async function validatePlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    none("platform validate", arguments_);
    await Root.validate(environment);
}

/** Format every deployment's root. */
export async function formatPlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    none("platform format", arguments_);
    await Root.format(false, environment);
}

/** Refuse unformatted deployment roots. */
export async function checkPlatformFormat(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    none("platform format-check", arguments_);
    await Root.format(true, environment);
}

/** Initialize a deployment's root against its remote state. */
export async function initializePlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    await new Root(arguments_, environment).operate("init");
}

/** Plan a deployment's root. */
export async function diffPlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    await new Root(arguments_, environment).operate("diff");
}

/** Apply one root, or deploy a whole environment: its roots in order, then its Workers. */
export async function deployPlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // deploy a whole environment
    const [name] = arguments_;
    if (arguments_.length === 1 && name !== undefined && !Root.isShared(name)) {
        await new Cloudflare(Deployment.cloudflare(name, environment)).deploy();
    }
    // apply one root
    else {
        await new Root(arguments_, environment).operate("deploy");
    }
}

/** Publish a deployed universe's releases into the platform accounts' forge, as its workflow. */
export async function publishPlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // refuse arguments beyond the universe
    const [name, ...rest] = arguments_;
    if (rest.length > 0) {
        throw new Error(
            `platform publish takes a universe, then nothing more: '${rest.join(" ")}'`,
        );
    }

    // sign in as the workflow's trusted publisher, by the job's OIDC token for the token endpoint
    const deployment = Deployment.cloudflare(name, environment);
    const send = (request: Request) => environment.network.fetch(request);
    const signer = { assert: (endpoint: string) => ActionsToken.request(endpoint, environment) };
    const session = new Session(
        new IssuerCredential(signer, deployment.universe.issuer, send, environment),
        { universe: deployment.universe.origin, send, bookmark: new Bookmark(), environment },
    );

    // publish the checkout's releases under the universe's tag, pointing latest, which installs follow
    await using build = await PlatformBuild.build(CHECKOUT, environment);
    await build.publish(session, deployment.universe.origin, deployment.tag, {
        tags: [LATEST_TAG],
    });
}

/** Build the checkout's first-party releases into its store. */
export async function buildPlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    none("platform build", arguments_);
    await using _build = await PlatformBuild.build(CHECKOUT, environment);
}

/** Seal the keys a deployed universe's custody record lacks, as the operator does once per key. */
export async function sealCustody(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // refuse arguments beyond the universe
    const [name, ...rest] = arguments_;
    if (rest.length > 0) {
        throw new Error(`custody seal takes a universe, then nothing more: '${rest.join(" ")}'`);
    }
    await new Cloudflare(Deployment.cloudflare(name, environment)).seal((title, line) =>
        printStatus(title, line, environment),
    );
}

/** Refuse any argument to a command that takes none. */
function none(name: string, arguments_: readonly string[]): void {
    if (arguments_.length > 0) {
        throw new Error(`'${name}' takes no arguments`);
    }
}

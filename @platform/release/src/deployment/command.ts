import type { Environment, HostCapability } from "@destack/environment";
import { printStatus } from "../distribution/output.ts";
import { SERVER } from "../local/command.ts";
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
    if (arguments_.length === 1 && name !== undefined && !Root.isStageless(name)) {
        await Deployment.deployed(name, environment).deploy((title, line) =>
            printStatus(title, line, environment),
        );
    }
    // apply one root
    else {
        await new Root(arguments_, environment).operate("deploy");
    }
}

/** Publish a deployed stage's releases into the platform accounts' forge, as its workflow. */
export async function publishPlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // refuse arguments beyond the stage
    const [name, ...rest] = arguments_;
    if (rest.length > 0) {
        throw new Error(`platform publish takes a stage, then nothing more: ${rest.join(" ")}`);
    }
    await Deployment.deployed(name, environment).publish((title, line) =>
        printStatus(title, line, environment),
    );
}

/** Seal the keys a deployed stage's custody record lacks, as the operator does once per key. */
export async function sealCustody(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // refuse arguments beyond the stage
    const [name, ...rest] = arguments_;
    if (rest.length > 0) {
        throw new Error(`custody seal takes a stage, then nothing more: ${rest.join(" ")}`);
    }
    await Deployment.deployed(name, environment).seal((title, line) =>
        printStatus(title, line, environment),
    );
}

/** Bundle a stage's Workers and build their uploads without deploying. */
export async function rehearsePlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // refuse arguments beyond the stage
    const [name, ...rest] = arguments_;
    if (rest.length > 0) {
        throw new Error(`platform rehearse takes a stage, then nothing more: ${rest.join(" ")}`);
    }
    await Deployment.deployed(name, environment).rehearse((title, line) =>
        printStatus(title, line, environment),
    );
}

/** Serve the local Workers under workerd over a fresh database, and check they answer. */
export async function testPlatform(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    none("platform e2e", arguments_);
    await Deployment.of("local", environment).e2e(SERVER, (title, line) =>
        printStatus(title, line, environment),
    );
}

/** Refuse any argument to a command that takes none. */
function none(name: string, arguments_: readonly string[]): void {
    if (arguments_.length > 0) {
        throw new Error(`${name} takes no arguments`);
    }
}

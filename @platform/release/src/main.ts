import type { Environment, HostCapability } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import { provisionAppleCredential } from "./signing/credential.ts";
import { archiveBackup } from "./key/archive.ts";
import { buildDistribution } from "./distribution/build.ts";
import { developDistribution } from "./distribution/develop.ts";
import { buildIcons } from "./distribution/icon.ts";
import { verifyInstallation } from "./distribution/install.ts";
import { openDistribution } from "./distribution/open.ts";
import { packDistribution } from "./distribution/pack.ts";
import { verifyDistribution } from "./distribution/verify.ts";
import { provisionGithubCredential } from "./github/credential.ts";
import { prepareRelease } from "./release/prepare.ts";
import { recordRelease } from "./release/record.ts";
import { buildInstaller } from "./installer/build.ts";
import { runCeremony } from "./key/ceremony.ts";
import { enrollKey } from "./key/enroll.ts";
import { generateKeys } from "./key/generate.ts";
import { rehearseKey } from "./key/rehearse.ts";
import { checkExpiry } from "./repository/expiry.ts";
import { renewUpdate } from "./repository/renew.ts";
import { signUpdate } from "./repository/sign.ts";
import { publishUpdate } from "./repository/upload.ts";
import { verifyUpdate } from "./repository/verify.ts";
import { signPlatform } from "./signing/sign.ts";
import { verifyAttestations } from "./key/attestation.ts";
import {
    checkPlatformFormat,
    deployPlatform,
    diffPlatform,
    formatPlatform,
    initializePlatform,
    rehearsePlatform,
    publishPlatform,
    sealCustody,
    testPlatform,
    validatePlatform,
} from "./deployment/command.ts";
import { resetLocal, serveLocal } from "./local/command.ts";

/** A release command: it runs on the host with the words after its noun and verb. */
type Command = (
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
) => Promise<void>;

/** The release commands by their noun and verb, each checking its own arguments. */
const COMMANDS: Readonly<Record<string, Command>> = {
    "distribution build": buildDistribution,
    "distribution pack": packDistribution,
    "distribution icon": buildIcons,
    "distribution develop": developDistribution,
    "distribution open": openDistribution,
    "distribution verify": verifyDistribution,
    "distribution install-check": verifyInstallation,

    "installer build": buildInstaller,

    "update sign": signUpdate,
    "update publish": publishUpdate,
    "update renew": renewUpdate,
    "update verify": verifyUpdate,
    "update expiry": checkExpiry,

    "key ceremony": runCeremony,
    "key enroll": enrollKey,
    "key generate": generateKeys,
    "key rehearse": rehearseKey,
    "key attest": verifyAttestations,

    "key backup": archiveBackup,

    "signing sign": signPlatform,

    "release prepare": prepareRelease,
    "release record": recordRelease,
    "github credential": provisionGithubCredential,

    "signing credential": provisionAppleCredential,

    "platform validate": validatePlatform,
    "platform format": formatPlatform,
    "platform format-check": checkPlatformFormat,
    "platform init": initializePlatform,
    "platform diff": diffPlatform,
    "platform deploy": deployPlatform,
    "platform publish": publishPlatform,
    "platform rehearse": rehearsePlatform,
    "custody seal": sealCustody,
    "platform e2e": testPlatform,

    "local serve": serveLocal,
    "local reset": resetLocal,
};

/** The host every command runs on. */
const system = BunEnvironment.system;

/** The noun and verb the command line requests, followed by the command's arguments. */
const [noun, verb, ...requestedArguments] = system.process.arguments;

/** The command the noun and verb select. */
const selected = COMMANDS[`${noun} ${verb}`];

// run the selected command
if (selected !== undefined) {
    await selected(system, requestedArguments);
}
// refuse an unknown noun and verb
else {
    throw new Error(`use ${Object.keys(COMMANDS).join(", ")}`);
}

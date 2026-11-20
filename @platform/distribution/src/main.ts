import { BunEnvironment } from "@destack/environment/bun";
import { Command } from "./distribution/output.ts";
import { provisionAppleCredential } from "./signing/credential.ts";
import { archiveBackup } from "./key/archive.ts";
import { buildDistribution } from "./distribution/build.ts";
import { developDistribution } from "./distribution/develop.ts";
import { buildIcons } from "./distribution/icon.ts";
import { verifyInstallation } from "./distribution/install.ts";
import { openDistribution } from "./distribution/open.ts";
import { packDistribution } from "./distribution/pack.ts";
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

/** The distribution commands by their noun and verb, each checking its own arguments. */
const COMMANDS: Readonly<Record<string, Command>> = {
    "distribution build": buildDistribution,
    "distribution pack": packDistribution,
    "distribution icon": buildIcons,
    "distribution develop": (environment) => developDistribution(environment),
    "distribution open": openDistribution,
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
};

// run the command the noun and verb select
await Command.run(COMMANDS, "distribution", BunEnvironment.system);

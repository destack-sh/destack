import { BunEnvironment } from "@destack/environment/bun";
import { Command } from "@platform/distribution";
import {
    buildPlatform,
    checkPlatformFormat,
    deployPlatform,
    diffPlatform,
    formatPlatform,
    initializePlatform,
    publishPlatform,
    sealCustody,
    validatePlatform,
} from "./deployment/command.ts";
import { cliLocal, desktopLocal, publishLocal, resetLocal, serveLocal } from "./local/command.ts";
import { deploySite } from "./site/site.ts";

/** The deployment commands by their noun and verb, each checking its own arguments. */
const COMMANDS: Readonly<Record<string, Command>> = {
    "platform validate": validatePlatform,
    "platform format": formatPlatform,
    "platform format-check": checkPlatformFormat,
    "platform init": initializePlatform,
    "platform diff": diffPlatform,
    "platform build": buildPlatform,
    "platform deploy": deployPlatform,
    "platform publish": publishPlatform,
    "custody seal": sealCustody,

    "site deploy": deploySite,

    "local serve": serveLocal,
    "local reset": resetLocal,
    "local publish": publishLocal,
    "local desktop": desktopLocal,
    "local cli": cliLocal,
};

// run the command the noun and verb select
await Command.run(COMMANDS, "deployment", BunEnvironment.system);

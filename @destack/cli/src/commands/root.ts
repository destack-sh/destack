import type {} from "@destack/package/import-meta";
import type { Command } from "../command/command.ts";
import type { CommandContext } from "../command/context.ts";
import { applyCommand, approveCommand, planCommand, rejectCommand } from "./apply.ts";
import { checkCommand, fixCommand, testCommand } from "./check.ts";
import { buildCommand, devCommand, publishCommand } from "./dev.ts";
import { gitCommand } from "./git.ts";
import { installCommand, uninstallCommand } from "./install.ts";
import { loginCommand, logoutCommand, whoamiCommand } from "./login.ts";
import { newCommand } from "./new.ts";
import { nounCommand } from "./noun.ts";
import { openCommand } from "./open.ts";
import { completionCommand } from "./completion.ts";
import { selfCommand } from "./self.ts";
import { settingCommand, settingsCommand } from "./setting.ts";
import { acceptCommand, invitedCommand, shareCommand } from "./share.ts";
import { switchCommand } from "./switch.ts";
import { tagCommand } from "./tag.ts";

/** The journeys the root's help lists its commands under, in the order a person meets them. */
const JOURNEYS = [
    { title: "you", commands: ["login", "logout", "whoami", "self", "completion"] },
    { title: "spaces", commands: ["switch"] },
    {
        title: "apps",
        commands: ["install", "uninstall", "open", "apply", "plan", "approve", "reject"],
    },
    { title: "share", commands: ["share", "invited", "accept"] },
    {
        title: "build",
        commands: ["new", "dev", "check", "fix", "test", "build", "publish", "tag", "git"],
    },
    { title: "settings", commands: ["setting", "settings"] },
] as const;

/** The root's commands by their words. */
const COMMANDS = {
    // the person and the space the terminal acts in
    login: loginCommand,
    logout: logoutCommand,
    whoami: whoamiCommand,
    switch: switchCommand,

    // the nouns taking more than their type's methods
    setting: settingCommand,
    settings: settingsCommand,

    // the space's packages, and its stack
    install: installCommand,
    uninstall: uninstallCommand,
    share: shareCommand,
    invited: invitedCommand,
    accept: acceptCommand,
    apply: applyCommand,
    plan: planCommand,
    approve: approveCommand,
    reject: rejectCommand,

    // this computer's checkouts and their repositories
    new: newCommand,
    dev: devCommand,
    build: buildCommand,
    check: checkCommand,
    fix: fixCommand,
    test: testCommand,
    publish: publishCommand,
    tag: tagCommand,
    git: gitCommand,

    // the desktop, this distribution and its completion
    open: openCommand,
    self: selfCommand,
    completion: completionCommand,
} as const satisfies Readonly<Record<string, Command>>;

/** The command line's root: its commands, and the nouns of the space it acts in. */
export const root: Command = {
    description: `Destack ${import.meta.destack.package.version}: destack <nouns> to list, destack <noun> <id> [method] to get and call`,
    commands: COMMANDS,
    journeys: JOURNEYS,
    // find a noun the stores describe, listed qualified where a root command takes its word
    resolve: async (word, context) => nounCommand(await context.catalog(), word),
    names: async (context) => (await context.catalog()).plurals(Object.keys(COMMANDS)),
    unavailable: (context) => unavailable(context),
};

/** Explain the nouns a command line lacks: signed out, or outside a space. */
async function unavailable(context: CommandContext): Promise<string | undefined> {
    // point a signed-out person at signing in, and one outside a space at switching to one
    if ((await context.me()) === undefined) {
        return "sign in with destack login for the universe's and spaces' commands";
    } else if ((await context.space()) === undefined) {
        return "switch to a space with destack switch <space> for its commands";
    }

    return undefined;
}

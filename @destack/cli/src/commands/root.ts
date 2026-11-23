import type {} from "@destack/package/import-meta";
import type { Command } from "../command/command.ts";
import type { CommandContext } from "../command/context.ts";

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
    login: {
        description:
            "Sign in or up with a device code, enrolling this machine; --browser signs in through the browser, --email with a mailed code, which --handle signs a new person up with",
        load: async (context) => (await context.module(() => import("./login.ts"))).loginCommand,
    },
    logout: {
        description: "Sign out of this machine",
        load: async (context) => (await context.module(() => import("./login.ts"))).logoutCommand,
    },
    whoami: {
        description: "Show the person signed in on this machine and the space the terminal acts in",
        load: async (context) => (await context.module(() => import("./login.ts"))).whoamiCommand,
    },
    switch: {
        description:
            "Switch the space command lines act in, stored as this computer's space setting",
        load: async (context) => (await context.module(() => import("./switch.ts"))).switchCommand,
    },

    // the nouns taking more than their type's methods
    setting: {
        description: "Get, set or reset a setting at the scope --scope selects",
        load: async (context) =>
            (await context.module(() => import("./setting.ts"))).settingCommand,
    },
    settings: {
        description: "List the setting values at the scope --scope selects",
        load: async (context) =>
            (await context.module(() => import("./setting.ts"))).settingsCommand,
    },

    // the space's packages, and its stack
    install: {
        description:
            "Install a package, following its latest release or a tag, or staying on a version",
        load: async (context) =>
            (await context.module(() => import("./install.ts"))).installCommand,
    },
    uninstall: {
        description: "Uninstall a package, or the installation of a name, deleting its resources",
        load: async (context) =>
            (await context.module(() => import("./install.ts"))).uninstallCommand,
    },
    share: {
        description: "Invite a person to the space, by @handle or email, as a role",
        load: async (context) => (await context.module(() => import("./share.ts"))).shareCommand,
    },
    invited: {
        description: "List the invitations to you",
        load: async (context) => (await context.module(() => import("./share.ts"))).invitedCommand,
    },
    accept: {
        description: "Accept an invitation to you, which destack invited lists",
        load: async (context) => (await context.module(() => import("./share.ts"))).acceptCommand,
    },
    apply: {
        description: "Apply a checkout's stack to the space, following it until applied",
        load: async (context) => (await context.module(() => import("./apply.ts"))).applyCommand,
    },
    plan: {
        description:
            "Show an installation, the space's stack by default, its resources and the plan it waits on",
        load: async (context) => (await context.module(() => import("./apply.ts"))).planCommand,
    },
    approve: {
        description: "Approve the plan an installation, the space's stack by default, waits on",
        load: async (context) => (await context.module(() => import("./apply.ts"))).approveCommand,
    },
    reject: {
        description: "Reject the plan an installation, the space's stack by default, waits on",
        load: async (context) => (await context.module(() => import("./apply.ts"))).rejectCommand,
    },

    // this computer's checkouts and their repositories
    new: {
        description:
            "Make a package from a template in a new repository, checked out under ~/Destack",
        load: async (context) => (await context.module(() => import("./new.ts"))).newCommand,
    },
    dev: {
        description: "Preview a checkout's package live against the space",
        load: async (context) => (await context.module(() => import("./dev.ts"))).devCommand,
    },
    build: {
        description: "Build a checkout's package in the space, following its phases",
        load: async (context) => (await context.module(() => import("./dev.ts"))).buildCommand,
    },
    check: {
        description: "Check a package's formatting, types and lints with its pinned toolchain",
        load: async (context) => (await context.module(() => import("./check.ts"))).checkCommand,
    },
    fix: {
        description: "Write a package's generated settings, format it and fix its lints",
        load: async (context) => (await context.module(() => import("./check.ts"))).fixCommand,
    },
    test: {
        description:
            "Run a package's tests with its pinned toolchain, unit and scenario by default",
        load: async (context) => (await context.module(() => import("./check.ts"))).testCommand,
    },
    publish: {
        description: "Build a checkout's package in the space and publish it to the registry",
        load: async (context) => (await context.module(() => import("./dev.ts"))).publishCommand,
    },
    tag: {
        description: "Point a package's distribution tag at one of its releases",
        load: async (context) => (await context.module(() => import("./tag.ts"))).tagCommand,
    },
    git: {
        description: "Push and pull Destack repositories with git as you",
        load: async (context) => (await context.module(() => import("./git.ts"))).gitCommand,
    },

    // the desktop, this distribution and its completion
    open: {
        description:
            "Open a link, or an object by its identifier, in a desktop window or the browser",
        load: async (context) => (await context.module(() => import("./open.ts"))).openCommand,
    },
    self: {
        description: "Install, start, stop, update and uninstall this Destack distribution",
        load: async (context) => (await context.module(() => import("./self.ts"))).selfCommand,
    },
    completion: {
        description: "Print the completion script of a shell: bash, zsh or fish",
        load: async (context) =>
            (await context.module(() => import("./completion.ts"))).completionCommand,
    },
} as const satisfies Readonly<Record<string, Command>>;

/** The command line's root: its commands, and the nouns of the space it acts in. */
export const root: Command = {
    description: `Destack ${import.meta.destack.package.version}: destack <nouns> to list, destack <noun> <id> [method] to get and call`,
    commands: COMMANDS,
    journeys: JOURNEYS,
    // find a noun the stores describe, listed qualified where a root command takes its word
    resolve: async (word, context) =>
        (await context.module(() => import("./noun.ts"))).nounCommand(
            await context.catalog(),
            word,
        ),
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

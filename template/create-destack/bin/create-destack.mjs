#!/usr/bin/env node

import { access, copyFile, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { constants as fsConstants, readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { createInterface } from "node:readline/promises";
import { fileURLToPath } from "node:url";

const DEFAULT_TEMPLATE = "app";
const DEFAULT_TARGET_DIRECTORY = "destack-app";
const TEMPLATES = new Map([
    ["app", "Application starter with src/main.ds and destack.json"],
    ["blank", "Minimal starter with package and source folder"],
]);

/**
 * Read the package version from the local package manifest.
 */
function readPackageVersion() {
    // load package metadata from this package directory
    const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
    const packageJsonPath = path.resolve(scriptDirectory, "../package.json");
    const packageJson = readFileSync(packageJsonPath, "utf8");
    const packageData = JSON.parse(packageJson);
    const packageVersion = packageData.version;

    // require a valid version for deterministic cli output
    if (typeof packageVersion !== "string" || packageVersion.trim().length === 0) {
        throw new Error(`missing version in ${packageJsonPath}`);
    }

    return packageVersion;
}

const VERSION = readPackageVersion();

/**
 * Print usage help for the initializer command.
 */
function printHelp() {
    // render template names for the usage line
    const templateNames = [...TEMPLATES.keys()].join("|");

    // print help content
    console.log(`create-destack v${VERSION}`);
    console.log("");
    console.log("Usage:");
    console.log(`  npm create destack@latest [target-directory] [-- --template <${templateNames}>] [--yes]`);
    console.log("");
    console.log("Options:");
    console.log(`  -t, --template <name>      Template to use (default: ${DEFAULT_TEMPLATE})`);
    console.log("  -p, --package-manager      Package manager for next steps (npm|pnpm|yarn|bun)");
    console.log("  --overwrite                Overwrite files in a non-empty target directory");
    console.log("  --dry-run                  Print planned actions without writing files");
    console.log("  -y, --yes                  Skip prompts");
    console.log("  -h, --help                 Show help");
}

/**
 * Parse cli arguments into a normalized options object.
 */
function parseArgs(argv) {
    // defaults
    let targetDirectory = "";
    let template = DEFAULT_TEMPLATE;
    let is_yes = false;
    let is_dry_run = false;
    let is_overwrite = false;
    let is_template_explicit = false;
    let packageManager = "";

    // scan args once and capture flags and positional values
    for (let index = 0; index < argv.length; index += 1) {
        const argument = argv[index];

        // exit early when help was requested
        if (argument === "-h" || argument === "--help") {
            return { is_help: true };
        }

        // enable non interactive mode
        if (argument === "-y" || argument === "--yes") {
            is_yes = true;
            continue;
        }

        // enable dry run output only mode
        if (argument === "--dry-run") {
            is_dry_run = true;
            continue;
        }

        // allow overwriting non empty target directories
        if (argument === "--overwrite") {
            is_overwrite = true;
            continue;
        }

        // capture explicit template choice
        if (argument === "-t" || argument === "--template") {
            template = argv[index + 1] ?? DEFAULT_TEMPLATE;
            is_template_explicit = true;
            index += 1;
            continue;
        }

        // capture explicit package manager choice
        if (argument === "--package-manager" || argument === "-p") {
            packageManager = (argv[index + 1] ?? "").trim();
            index += 1;
            continue;
        }

        // use the first positional arg as target directory
        if (!argument.startsWith("-") && targetDirectory.length === 0) {
            targetDirectory = argument;
        }
    }

    // return normalized parse result
    return {
        is_help: false,
        is_dry_run,
        is_overwrite,
        is_template_explicit,
        is_yes,
        packageManager,
        targetDirectory,
        template,
    };
}

/**
 * Normalize a target directory string for filesystem operations.
 */
function formatTargetDirectory(directory) {
    return directory.trim().replace(/\/+$/g, "") || ".";
}

/**
 * Assert that a template name is supported by this initializer.
 */
function assertTemplate(templateName) {
    if (!TEMPLATES.has(templateName)) {
        const supported = [...TEMPLATES.keys()].join(", ");
        throw new Error(`unknown template: ${templateName}. Supported templates: ${supported}`);
    }
}

/**
 * Resolve package manager from explicit args or npm user agent.
 */
function detectPackageManager(explicitPackageManager) {
    // prefer explicitly passed value
    const normalizedExplicitPackageManager = explicitPackageManager.toLowerCase();
    if (normalizedExplicitPackageManager.length > 0) {
        return normalizedExplicitPackageManager;
    }

    // otherwise infer from npm user agent prefix
    const userAgent = process.env.npm_config_user_agent ?? "";
    if (userAgent.startsWith("pnpm/")) {
        return "pnpm";
    }

    if (userAgent.startsWith("yarn/")) {
        return "yarn";
    }

    if (userAgent.startsWith("bun/")) {
        return "bun";
    }

    if (userAgent.startsWith("npm/")) {
        return "npm";
    }

    // default to npm for unknown agents
    return "npm";
}

/**
 * Resolve install and dev commands for a package manager.
 */
function resolveNextStepCommands(packageManager) {
    switch (packageManager) {
        case "pnpm":
            return { install: "pnpm install", dev: "pnpm dev" };
        case "yarn":
            return { install: "yarn", dev: "yarn dev" };
        case "bun":
            return { install: "bun install", dev: "bun run dev" };
        case "npm":
            return { install: "npm install", dev: "npm run dev" };
        default:
            throw new Error(`unsupported package manager: ${packageManager}`);
    }
}

/**
 * Validate whether a string is a legal npm package name.
 */
function isValidPackageName(packageName) {
    return /^(?:@[a-z0-9-*~][a-z0-9-*._~]*\/[a-z0-9-~][a-z0-9-._~]*|[a-z0-9-~][a-z0-9-._~]*)$/.test(
        packageName,
    );
}

/**
 * Convert an arbitrary string into a safer npm package name.
 */
function toValidPackageName(packageName) {
    return packageName
        .toLowerCase()
        .trim()
        .replace(/\s+/g, "-")
        .replace(/^[._]+/, "")
        .replace(/[^a-z0-9-~]+/g, "-")
        .replace(/^-+/, "")
        .replace(/-+$/, "");
}

/**
 * Derive a package name from the target directory.
 */
function derivePackageName(targetDirectory) {
    // use cwd basename when writing into current directory
    if (targetDirectory === ".") {
        return path.basename(process.cwd());
    }

    // otherwise use the target directory basename
    return path.basename(targetDirectory);
}

/**
 * Assert that a template directory can be read.
 */
async function ensureTemplateExists(templateDirectory) {
    try {
        await access(templateDirectory, fsConstants.R_OK);
    }
    catch {
        throw new Error(`template directory does not exist: ${templateDirectory}`);
    }
}

/**
 * Return whether a directory does not contain any entries.
 */
async function isDirectoryEmpty(directory) {
    try {
        const entries = await readdir(directory);
        return entries.length === 0;
    }
    catch {
        return true;
    }
}

/**
 * Delete all entries inside a directory.
 */
async function emptyDirectory(directory) {
    const entries = await readdir(directory);

    for (const entry of entries) {
        await rm(path.join(directory, entry), { recursive: true, force: true });
    }
}

/**
 * Copy one directory tree recursively to a destination.
 */
async function copyDirectory(sourceDirectory, destinationDirectory) {
    // create destination path first
    await mkdir(destinationDirectory, { recursive: true });

    // walk source directory entries
    const entries = await readdir(sourceDirectory, { withFileTypes: true });
    for (const entry of entries) {
        const sourcePath = path.join(sourceDirectory, entry.name);
        const destinationPath = path.join(destinationDirectory, entry.name);

        // recurse into subdirectories
        if (entry.isDirectory()) {
            await copyDirectory(sourcePath, destinationPath);
            continue;
        }

        // copy regular files
        await copyFile(sourcePath, destinationPath);
    }
}

/**
 * Write the resolved package name into the generated package file.
 */
async function writePackageName(projectDirectory, packageName) {
    // skip templates that do not ship a package manifest
    const packageJsonPath = path.join(projectDirectory, "package.json");
    try {
        await access(packageJsonPath, fsConstants.F_OK);
    }
    catch {
        return;
    }

    // load package json and rewrite the name field
    const packageJson = await readFile(packageJsonPath, "utf8");
    const packageData = JSON.parse(packageJson);
    packageData.name = packageName;

    // persist normalized package json with trailing newline
    await writeFile(packageJsonPath, `${JSON.stringify(packageData, null, 2)}\n`, "utf8");

    // sync destack json when the template ships one
    const destackJsonPath = path.join(projectDirectory, "destack.json");
    try {
        await access(destackJsonPath, fsConstants.F_OK);
    }
    catch {
        return;
    }

    // load destack json and rewrite the name field
    const destackJson = await readFile(destackJsonPath, "utf8");
    const destackData = JSON.parse(destackJson);
    destackData.name = packageName;

    // persist normalized destack json with trailing newline
    await writeFile(destackJsonPath, `${JSON.stringify(destackData, null, 2)}\n`, "utf8");
}

/**
 * Print post init commands for the selected package manager.
 */
function printNextSteps(targetDirectory, nextSteps) {
    // print section header
    console.log("\nNext steps:");

    // print directory change only for non dot targets
    if (targetDirectory !== ".") {
        console.log(`  cd ${targetDirectory}`);
    }

    // print install and run commands
    console.log(`  ${nextSteps.install}`);
    console.log(`  ${nextSteps.dev}`);
}

/**
 * Prompt for target and template when interactive mode is enabled.
 */
async function promptForInitialization(initialTargetDirectory, initialTemplate, is_template_prompt_enabled) {
    // open prompt reader
    const reader = createInterface({
        input: process.stdin,
        output: process.stdout,
    });

    try {
        // seed with incoming defaults
        let targetDirectory = initialTargetDirectory;
        let templateName = initialTemplate;

        // prompt for target directory when not provided
        if (targetDirectory.length === 0) {
            const answer = await reader.question(`Target directory (${DEFAULT_TARGET_DIRECTORY}): `);
            targetDirectory = formatTargetDirectory(answer.length > 0 ? answer : DEFAULT_TARGET_DIRECTORY);
        }

        // prompt for template when no explicit template flag was passed
        if (is_template_prompt_enabled) {
            const supported = [...TEMPLATES.keys()].join(", ");
            const answer = await reader.question(`Template (${templateName}) [${supported}]: `);
            const candidate = answer.trim();

            if (candidate.length > 0) {
                templateName = candidate;
            }
        }

        return { targetDirectory, templateName };
    }
    finally {
        // always close prompt reader
        reader.close();
    }
}

/**
 * Prompt whether a non empty directory can be overwritten.
 */
async function promptForOverwrite(targetDirectory) {
    // open prompt reader
    const reader = createInterface({
        input: process.stdin,
        output: process.stdout,
    });

    try {
        // ask overwrite confirmation and map to bool
        const answer = await reader.question(
            `Target directory "${targetDirectory}" is not empty. Overwrite existing files? (y/N): `,
        );
        return answer.trim().toLowerCase() === "y";
    }
    finally {
        // always close prompt reader
        reader.close();
    }
}

/**
 * Prompt for a valid package name when derived name is invalid.
 */
async function promptForPackageName(initialPackageName) {
    // open prompt reader
    const reader = createInterface({
        input: process.stdin,
        output: process.stdout,
    });

    try {
        // use sanitized name as default suggestion
        const suggestedName = toValidPackageName(initialPackageName) || "destack-app";
        const answer = await reader.question(`Package name (${suggestedName}): `);
        const candidateName = answer.trim().length > 0 ? answer.trim() : suggestedName;

        // enforce npm package naming rules
        if (!isValidPackageName(candidateName)) {
            throw new Error(`invalid package name: ${candidateName}`);
        }

        return candidateName;
    }
    finally {
        // always close prompt reader
        reader.close();
    }
}

/**
 * Execute the initializer command.
 */
async function main() {
    // parse cli arguments
    const parsed = parseArgs(process.argv.slice(2));

    // return early for help mode
    if (parsed.is_help) {
        printHelp();
        return;
    }

    // resolve target and runtime configuration
    let targetDirectory = formatTargetDirectory(parsed.targetDirectory || DEFAULT_TARGET_DIRECTORY);
    let templateName = parsed.template;
    const packageManager = detectPackageManager(parsed.packageManager);
    const nextSteps = resolveNextStepCommands(packageManager);

    // prompt for missing values when interactive mode is enabled
    if (!parsed.is_yes) {
        const prompted = await promptForInitialization(
            parsed.targetDirectory,
            parsed.is_template_explicit ? templateName : DEFAULT_TEMPLATE,
            !parsed.is_template_explicit,
        );
        targetDirectory = formatTargetDirectory(prompted.targetDirectory);
        templateName = prompted.templateName;
    }

    // validate template selection
    assertTemplate(templateName);

    // derive and validate package name
    let packageName = derivePackageName(targetDirectory);
    if (!isValidPackageName(packageName)) {
        // in non interactive mode: sanitize and validate
        if (parsed.is_yes) {
            packageName = toValidPackageName(packageName) || "destack-app";

            if (!isValidPackageName(packageName)) {
                throw new Error(`invalid package name derived from target directory: ${packageName}`);
            }
        }
        // in interactive mode: ask user for a valid package name
        else {
            packageName = await promptForPackageName(packageName);
        }
    }

    // resolve and validate template directory
    const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
    const templateDirectory = path.resolve(scriptDirectory, "../templates", templateName);
    await ensureTemplateExists(templateDirectory);

    // inspect target directory state
    const projectDirectory = path.resolve(process.cwd(), targetDirectory);
    const isEmpty = await isDirectoryEmpty(projectDirectory);

    // enforce overwrite rules for non empty directories
    let shouldOverwrite = parsed.is_overwrite;
    if (!isEmpty && !shouldOverwrite) {
        // fail fast in non interactive mode
        if (parsed.is_yes) {
            throw new Error(
                `target directory is not empty: ${projectDirectory}. Use --overwrite or run without --yes to confirm`,
            );
        }

        // ask for overwrite confirmation in interactive mode
        shouldOverwrite = await promptForOverwrite(targetDirectory);
        if (!shouldOverwrite) {
            throw new Error("operation cancelled");
        }
    }

    // print execution plan and exit during dry run mode
    if (parsed.is_dry_run) {
        console.log(`\n[dry-run] Would create ${targetDirectory} using template ${templateName}.`);
        console.log(`[dry-run] Package name: ${packageName}`);

        if (!isEmpty && shouldOverwrite) {
            console.log(`[dry-run] Would overwrite existing files in ${targetDirectory}.`);
        }

        printNextSteps(targetDirectory, nextSteps);
        return;
    }

    // create target directory before copying template content
    await mkdir(projectDirectory, { recursive: true });

    // clear target directory contents only for non dot targets
    if (targetDirectory !== "." && !isEmpty && shouldOverwrite) {
        await emptyDirectory(projectDirectory);
    }

    // copy template and patch package name
    await copyDirectory(templateDirectory, projectDirectory);
    await writePackageName(projectDirectory, packageName);

    // print success summary
    console.log(`\nCreated ${targetDirectory} using template ${templateName}.`);
    printNextSteps(targetDirectory, nextSteps);
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`create-destack: ${message}`);
    process.exit(1);
});

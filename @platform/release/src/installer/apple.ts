import { CHECKOUT } from "../distribution/distribution.ts";
import { dirname, join } from "node:path";
import { type Installer, releaseVersion } from "../distribution/index.ts";
import { MacSigning } from "../signing/apple.ts";
import { run } from "../distribution/command.ts";
import { Release, Universe } from "@destack/update/release";
import type { Environment, HostCapability } from "@destack/environment";
import { DISTRIBUTION_MANIFEST, DistributionManifest } from "@destack/package";
import { DISTRIBUTION_DIRECTORY } from "@destack/update";

/** The distribution's manifest in the application's resources, which lists each slice's images. */
const MANIFEST = join(DISTRIBUTION_DIRECTORY, DISTRIBUTION_MANIFEST);

/** One architecture's signed application unpacked from its archive. */
interface Slice {
    /** The distribution target of the archive. */
    target: string;
    /** The Mach-O architecture name. */
    architecture: string;
    /** The unpacked application. */
    application: string;
}

/** Build a universal application in a disk image, from signed applications or the archives. */
export async function buildMacInstaller(
    installer: Installer,
    environment: Environment<HostCapability>,
    signed: Readonly<Record<string, string>> = {},
): Promise<string> {
    // require macOS, and stage below a private temporary directory
    if (environment.host.os !== "darwin") {
        throw new Error("build the macOS application on macOS");
    }
    const { files } = environment;
    const version = releaseVersion(environment);
    const output = join(CHECKOUT, "dist", version);
    const staging = await files.makeTemporaryDirectory("destack-macos-");
    const image = join(staging, "image");
    const title = new Universe("production").title;
    const application = join(image, `${title}.app`);
    const signing = new MacSigning(environment);
    try {
        // start the universal application from the arm64 slice
        await files.mkdir(image);
        const slices = await unpackSlices(staging, output, version, signed, environment);
        const [primary] = slices;
        await run("ditto", [primary.application, application], environment);
        const contents = join(application, "Contents");

        // merge each executable of both slices, then their resources
        for (const name of ["MacOS/Destack", `Helpers/${Release.executable("darwin")}`]) {
            await mergeExecutable(name, contents, slices, environment);
        }
        await mergeResources(contents, slices, environment);
        await signing.sign(application);
        await signing.verify(application);

        // notarize the universal application with the signed ones in one submission
        await signing.notarize([application, ...Object.values(signed)]);
        const name = `destack-${version}-${installer.name}.${installer.format}`;

        return await createImage(title, name, image, staging, output, signing);
    } finally {
        await files.remove(staging, { recursive: true, force: true });
    }
}

/** Unpack the arm64 and x86_64 applications from their distribution archives. */
async function unpackSlices(
    staging: string,
    output: string,
    version: string,
    signed: Readonly<Record<string, string>>,
    environment: Environment<"files" | "process">,
): Promise<[Slice, Slice]> {
    // unpack both signed native applications without changing their compiled CLI payloads
    const slices: [Slice, Slice] = [
        {
            target: "aarch64-apple-darwin",
            architecture: "arm64",
            application: join(staging, "aarch64-apple-darwin", "Destack.app"),
        },
        {
            target: "x86_64-apple-darwin",
            architecture: "x86_64",
            application: join(staging, "x86_64-apple-darwin", "Destack.app"),
        },
    ];
    for (const slice of slices) {
        // take the signed application of a target given one
        const { target } = slice;
        const application = signed[target];
        if (application !== undefined) {
            slice.application = application;
            continue;
        }

        // unpack the application of any other target from its archive
        await environment.files.mkdir(join(staging, target));
        await run(
            "tar",
            [
                "-xzf",
                join(output, `destack-${version}-${target}.tar.gz`),
                "-C",
                join(staging, target),
                "Destack.app",
            ],
            environment,
        );
    }

    return slices;
}

/** Merge an executable of each slice into one universal executable. */
async function mergeExecutable(
    name: string,
    contents: string,
    slices: Slice[],
    environment: Environment<"process">,
): Promise<void> {
    // merge and verify both architectures
    const destination = join(contents, name);
    const executables = slices.map((slice) => join(slice.application, "Contents", name));
    const architectures = slices.map((slice) => slice.architecture);
    await run("lipo", ["-create", ...executables, "-output", destination], environment);
    await run("lipo", [destination, "-verify_arch", ...architectures], environment);
}

/** Merge each slice's resources into the application's, native packages side by side. */
async function mergeResources(
    contents: string,
    slices: Slice[],
    environment: Environment<"files">,
): Promise<void> {
    // copy each slice's files with their modes, merging the shared ones
    const { files } = environment;
    const merged = join(contents, "Resources");
    for (const { application } of slices) {
        const source = join(application, "Contents/Resources");
        const entries = await files.readDirectory(source, { recursive: true });
        for (const entry of entries.filter((each) => each.kind === "file")) {
            const from = join(source, entry.name);
            const target = join(merged, entry.name);

            // copy a file new to the application
            if (!(await files.exists(target))) {
                await files.mkdir(dirname(target), { recursive: true });
                await files.copyFile(from, target);
            }
            // merge the slices' manifests into each release's images on both platforms
            else if (entry.name === MANIFEST) {
                const [copied, kept] = [
                    await files.readTextFile(from),
                    await files.readTextFile(target),
                ];
                const manifest = DistributionManifest.merge(
                    DistributionManifest.parse(JSON.parse(kept)),
                    DistributionManifest.parse(JSON.parse(copied)),
                );
                await files.writeTextFile(target, JSON.stringify(manifest));
            }
            // refuse another shared file whose bytes differ between the slices
            else {
                const [copied, kept] = [await files.readFile(from), await files.readFile(target)];
                if (!Buffer.from(copied).equals(Buffer.from(kept))) {
                    throw new Error(`the slices' resources differ at '${entry.name}'`);
                }
            }
        }
    }
}

/** Create, verify and sign the disk image and move it into the output directory. */
async function createImage(
    title: string,
    name: string,
    image: string,
    staging: string,
    output: string,
    signing: MacSigning,
): Promise<string> {
    // let Finder provide the standard installation interaction
    const { environment } = signing;
    await environment.files.symlink("/Applications", join(image, "Applications"));
    const temporary = join(staging, name);
    await run(
        "hdiutil",
        ["create", "-volname", title, "-srcfolder", image, "-format", "UDZO", temporary],
        environment,
    );

    // verify and sign the image before moving it into the output
    await run("hdiutil", ["verify", temporary], environment);
    await signing.image(temporary);
    await environment.files.rename(temporary, join(output, name));

    return join(output, name);
}

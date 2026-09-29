import { cp, mkdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";
import { run } from "./command.ts";

/** Exact upstream source corresponding to the installed native binding. */
const REVISION = "046e9cbf67d22491e8ecc941ec2891b02a9f3cad";
/** Native binding version whose release omits Intel macOS. */
const VERSION = "0.7.2";
/** Checked-out upstream source and native output supplied by the build job. */
const source = resolve(process.argv[2]);
/** Intel addon consumed by the executable compiler. */
const destination = resolve("dist/native/turso-x86_64-apple-darwin.node");

// verify both sides of the native ABI before compiling upstream source
/** Resolve the JavaScript binding installed by the workspace lockfile. */
const require = createRequire(import.meta.url);
/** Installed native package entrypoint. */
const entry = require.resolve("@tursodatabase/database");
/** Installed binding version. */
const manifest = JSON.parse(await readFile(join(entry, "../../package.json"), "utf8"));
/** Source revision reported by the checked-out upstream repository. */
const revision = Bun.spawn(["git", "rev-parse", "HEAD"], {
    cwd: source,
    stdout: "pipe",
    stderr: "inherit",
    timeout: 10000,
});
/** Exact upstream commit selected for this native build. */
const actual = (await new Response(revision.stdout).text()).trim();
if ((await revision.exited) !== 0 || actual !== REVISION || manifest.version !== VERSION) {
    throw new Error("turso source revision and installed binding version do not match");
}

// use upstream's locked dependencies and release profile without changing its source
await run(
    "cargo",
    [
        "build",
        "--locked",
        "--package",
        "turso_node",
        "--lib",
        "--profile",
        "release-official",
        "--target",
        "x86_64-apple-darwin",
    ],
    source,
);
await mkdir(resolve("dist/native"), { recursive: true });
await cp(
    join(source, "target/x86_64-apple-darwin/release-official/libturso_node.dylib"),
    destination,
);
console.log(destination);

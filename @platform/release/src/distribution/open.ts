import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Release } from "@destack/update/release";
import { version } from "./index.ts";

/** Source repository containing the complete local distribution. */
const root = fileURLToPath(new URL("../../../../", import.meta.url));
/** Compiled distribution for the current platform. */
const directory = join(root, "dist", version, Release.target());
/** Native desktop entrypoint inside the compiled distribution. */
const executable =
    process.platform === "darwin"
        ? join(directory, "Destack.app/Contents/MacOS/Destack")
        : join(directory, "Destack", process.platform === "win32" ? "Destack.exe" : "Destack");
/** Interactive local desktop process. */
const child = Bun.spawn([executable], { stdin: "inherit", stdout: "inherit", stderr: "inherit" });
process.exitCode = await child.exited;

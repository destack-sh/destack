import { buildMacInstaller } from "./macos.ts";

// build the universal macOS installer
if (process.argv[2] !== undefined && process.argv[2] !== "macos") {
    throw new Error("select macos");
}
console.log(await buildMacInstaller());

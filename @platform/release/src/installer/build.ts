import { buildMacInstaller } from "./macos.ts";
import { buildWindowsInstaller } from "./windows.ts";

// build the native installer selected by the release job
if (process.argv[2] === "windows") {
    console.log(await buildWindowsInstaller());
}
// emit the uninstaller before the platform-signing job authenticates the complete payload
else if (process.argv[2] === "windows-prepare") {
    console.log(await buildWindowsInstaller(true));
}
// retain the universal macOS installer as the default native packaging command
else if (process.argv[2] === undefined || process.argv[2] === "macos") {
    console.log(await buildMacInstaller());
}
// reject unsupported packaging commands before selecting a platform
else {
    throw new Error("select macos, windows or windows-prepare");
}

import { cp, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { run } from "../distribution/command.ts";
import { version } from "../distribution/index.ts";
import { Release } from "@destack/update/release";

/** Package the complete Windows application in a per-user Setup executable. */
export async function buildWindowsInstaller(isPreparation = false): Promise<string> {
    if (process.platform !== "win32") {
        throw new Error("build the Windows installer on Windows");
    }
    // select one compiled Windows payload and its matching stable or nightly identity
    const target = "x86_64-pc-windows-msvc";
    const release = new Release(version, target);
    const root = fileURLToPath(new URL("../../../../", import.meta.url));
    const directory = join(root, "dist", version);
    const staging = await mkdtemp(join(tmpdir(), "destack-nsis-"));
    const title = release.channel === "stable" ? "Destack" : "Destack Nightly";
    const identity = release.channel === "stable" ? "destack" : "destack-nightly";
    const uninstaller = join(directory, target, "uninstaller");
    const output = isPreparation
        ? join(uninstaller, "prepare.exe")
        : join(directory, `destack-${version}-${target}.exe`);
    try {
        // retain platform-signed executable bytes and add the package installation descriptor
        const payload = join(staging, "Destack");
        await mkdir(directory, { recursive: true });
        await mkdir(uninstaller, { recursive: true });
        await cp(join(directory, target, "Destack"), payload, { recursive: true });
        await writeFile(
            join(payload, "installation.json"),
            JSON.stringify({ method: "nsis", version, target }) + "\n",
        );

        // embed Microsoft's authenticated Evergreen bootstrapper for clean Windows machines
        const webview = join(staging, "MicrosoftEdgeWebview2Setup.exe");
        if (!isPreparation) {
            await run(
                "powershell.exe",
                [
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    `
$ErrorActionPreference = 'Stop'
Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $env:DESTACK_WEBVIEW2
$signature = Get-AuthenticodeSignature -LiteralPath $env:DESTACK_WEBVIEW2
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.GetNameInfo([System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName, $false) -ne 'Microsoft Corporation') {
    throw 'invalid Microsoft WebView2 bootstrapper signature'
}
`,
                ],
                staging,
                { DESTACK_WEBVIEW2: webview },
            );
        }

        // compile the standard per-user installer without granting administrator privileges
        const prefix = "/D";
        await run("makensis", [
            "/WX",
            `${prefix}TITLE=${title}`,
            `${prefix}IDENTITY=${identity}`,
            `${prefix}VERSION=${version}`,
            `${prefix}OUTPUT=${output}`,
            `${prefix}PAYLOAD=${payload}`,
            ...(!isPreparation ? [`${prefix}WEBVIEW=${webview}`] : []),
            ...(isPreparation
                ? [`${prefix}UNINSTALLER_ONLY`]
                : [`${prefix}UNINSTALLER=${join(uninstaller, "Uninstall.exe")}`]),
            fileURLToPath(new URL("windows.nsi", import.meta.url)),
        ]);
        if (isPreparation) {
            await run(output, ["/S"]);
            await rm(output);
        }

        return isPreparation ? join(uninstaller, "Uninstall.exe") : output;
    } finally {
        await rm(staging, { recursive: true, force: true });
    }
}

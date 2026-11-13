import { Simulation } from "@destack/environment/simulation";
import type { OperatingSystem } from "@destack/environment";
import { expect, test } from "@destack/test";
import { Autostart } from "./autostart.ts";
import { LaunchAgent } from "./launchd.ts";
import { SystemdService } from "./systemd.ts";
import { testWorldOptions } from "@destack/test/world";

/** A machine directory with native configuration metacharacters. */
const DIRECTORY = '/tmp/destack & "dev" %state';

/** A machine executable with native configuration metacharacters. */
const EXECUTABLE = "/Applications/Destack & Dev.app/Contents/Helpers/destack-machine";

/** Locate the machine's registration on a host of an operating system, with the host. */
async function autostart(os: OperatingSystem) {
    // run on a world whose host reports the operating system, with only the variables the systems read
    const host = {
        os,
        architecture: "arm64",
        version: "1",
        hostname: "host",
        home: "/home/ada",
    } as const;
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: 0,
        host,
    });
    const environment = simulation.environment("host");
    const variables = { APPDATA: "C:/Users/ada/AppData/Roaming" };

    const commands = { ...environment, process: { ...environment.process, variables } };

    return { autostart: await Autostart.of(DIRECTORY, commands), commands };
}

test("describe a foreground machine with isolated state and bounded shutdown on each system, supervised but on Windows", async () => {
    // describe the machine as a launchd agent, a systemd unit and a startup shortcut
    const [mac, gnu, windows] = await Promise.all([
        autostart("darwin"),
        autostart("linux"),
        autostart("win32"),
    ]);
    const darwin = mac.autostart;
    const linux = gnu.autostart;
    expect({
        name: darwin.name,
        paths: [darwin.path, linux.path, windows.autostart.path],
        launchd: new LaunchAgent(darwin.name, DIRECTORY, mac.commands).describe(EXECUTABLE),
        systemd: new SystemdService(linux.name, DIRECTORY, gnu.commands).describe(EXECUTABLE),
        supervised: [darwin.isSupervised, linux.isSupervised, windows.autostart.isSupervised],
    }).toEqual({
        name: linux.name,
        paths: [
            `/home/ada/Library/LaunchAgents/${darwin.name}.plist`,
            `/home/ada/.config/systemd/user/${linux.name}.service`,
            `C:/Users/ada/AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Startup/${windows.autostart.name}.lnk`,
        ],
        launchd: `<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>Label</key><string>${darwin.name}</string>
<key>ProgramArguments</key><array><string>/Applications/Destack &amp; Dev.app/Contents/Helpers/destack-machine</string></array>
<key>EnvironmentVariables</key><dict><key>DESTACK_DIRECTORY</key><string>/tmp/destack &amp; &quot;dev&quot; %state</string></dict>
<key>RunAtLoad</key><true/>
<key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>
<key>ExitTimeOut</key><integer>20</integer>
<key>StandardOutPath</key><string>/tmp/destack &amp; &quot;dev&quot; %state/machine.log</string>
<key>StandardErrorPath</key><string>/tmp/destack &amp; &quot;dev&quot; %state/machine.log</string>
</dict></plist>
`,
        systemd: `[Unit]
Description=Destack machine

[Service]
Type=exec
ExecStart=:"/Applications/Destack & Dev.app/Contents/Helpers/destack-machine"
Environment="DESTACK_DIRECTORY=/tmp/destack & \\"dev\\" %%state"
Restart=on-failure
RestartSec=2
TimeoutStopSec=20

[Install]
WantedBy=default.target
`,
        supervised: [true, true, false],
    });
});

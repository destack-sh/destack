import { testSimulation } from "@destack/object/test";
import { expect, refusal, test } from "@destack/test";
import { Application } from "./application.ts";

/** Read the application of a simulated host of an operating system, running an executable. */
function application(os: "darwin" | "linux", executable: string): Application {
    const environment = testSimulation().environment("desktop");

    return new Application("/home/florian/.destack", {
        ...environment,
        host: { ...environment.host, os },
        process: { ...environment.process, executable },
    });
}

test("locate the desktop application beside its executable on each operating system", async () => {
    expect({
        mac: application("darwin", "/Applications/Destack.app/Contents/MacOS/destack").locate(),
        helper: application("linux", "/opt/destack/Destack/helpers/destack").locate(),
        bin: application("linux", "/opt/destack/bin/destack").locate(),
        outside: await refusal(() => application("darwin", "/usr/local/bin/destack").locate()),
        releases: [
            application("darwin", "/usr/local/bin/destack").locateIn("/releases/1.2.0"),
            application("linux", "/usr/local/bin/destack").locateIn("/releases/1.2.0"),
        ],
    }).toEqual({
        mac: "/Applications/Destack.app",
        helper: "/opt/destack/Destack",
        bin: "/opt/destack/Destack",
        outside: [
            "PRECONDITION_FAILED",
            "APPLICATION_BLOCKED",
            "cannot run the desktop: this command must run inside the Destack application",
        ],
        releases: ["/releases/1.2.0/Destack.app", "/releases/1.2.0/Destack"],
    });
});

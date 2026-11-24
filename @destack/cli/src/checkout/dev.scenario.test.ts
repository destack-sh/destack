import { PUBLISHER } from "@destack/machine/computer/test";
import { join } from "node:path";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { jsonLines, TerminalFixture } from "../test/terminal.ts";

/** The world of every scenario: the example organization publishing the starter template. */
const PUBLISHED = { organizations: [PUBLISHER], published: ["starter"] };

/** The lines a `dev --json` prints before a push at most: its installation's status lines. */
const PUSH_LINES = 40;

/** A printed build's state. */
const Progress = schema.looseObject({ state: schema.string() });

/** A printed build's summary. */
const Summary = schema.looseObject({
    space: schema.string(),
    package: schema.looseObject({ name: schema.string() }),
});

/** What `dev --json` prints of the installation and each commit it pushes, beside status lines. */
const Developed = schema.looseObject({
    id: schema.identifier("installation"),
    name: schema.string(),
    commit: schema.string().regex(/^[0-9a-f]{40}$/u),
});

/** Read the pushes a `dev --json` printed, leaving out its status lines. */
function pushes(output: string) {
    return jsonLines(output).flatMap((line) => {
        const parsed = Developed.safeParse(line);

        return parsed.success ? [parsed.data] : [];
    });
}

/** An installation as `installations --json` lists it. */
const Listed = schema.array(
    schema.looseObject({
        name: schema.string(),
        versionType: schema.string().nullable(),
        directory: schema.string().nullable(),
    }),
);

/** The identity of the scenarios' commits. */
const GIT_IDENTITY = ["-c", "user.name=Ada", "-c", "user.email=ada@example.test"] as const;

/** Run git in a working tree, answering its trimmed output. */
async function git(directory: string, ...command: readonly string[]): Promise<string> {
    const ran = testHost().process.spawn(["git", "-C", directory, ...GIT_IDENTITY, ...command]);
    const [output, code] = await Promise.all([new Response(ran.stdout).text(), ran.exited]);
    expect(code).toBe(0);

    return output.trim();
}

/** Make the owner's notes package from the starter template, committed and pushed by `new`. */
async function notes() {
    // make it in the person's home directory under the owner's handle
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" }, [], PUBLISHED);
    const directory = join(terminal.native.home, "notes");
    const name = `@${terminal.computer.world.owner.handle}/notes`;
    const template = ["--template", "@example/starter"];
    const made = await terminal.printed(["new", name, directory, ...template]);
    expect([made.code, made.error]).toEqual([0, ""]);

    // list the version type and directory of the notes installation
    const versions = async () => {
        const listed = await terminal.printed(["--space", "home", "installations", "--json"]);

        return Listed.parse(JSON.parse(listed.output))
            .filter((row) => row.name === "notes")
            .map((row) => [row.versionType, row.directory]);
    };

    return { terminal, directory, name, versions };
}

test("build a checkout's working tree at the commit its push answers, until the build ends", async () => {
    // build the new package's checkout in the owner's home
    const { terminal, directory, name } = await notes();
    const built = await terminal.printed(["--space", "home", "build", directory, "--json"]);
    expect([built.code, built.error]).toEqual([0, ""]);

    // show the build's states to its summary
    const lines = built.output.trim().split("\n");
    const summary = Summary.parse(JSON.parse(lines.at(-1) ?? ""));
    const states = lines.slice(0, -1).map((line) => Progress.parse(JSON.parse(line)).state);
    expect([built.code, built.error, states.at(-1), summary.package.name]).toEqual([
        0,
        "",
        "succeeded",
        name,
    ]);
});

test("develop a checkout: follow its reference, push each change, and stop on an interrupt", async () => {
    // develop the new package's checkout, which installs it following the checkout's reference
    const { terminal, directory, versions } = await notes();
    const job = terminal.start(["--space", "home", "dev", directory, "--json"]);
    const pushed = async (count: number) => {
        // wait line by line, refusing a job that printed no push past its next lines
        for (let lines = 1; pushes(job.output()).length < count; lines += 1) {
            if (lines > PUSH_LINES) {
                const { code, error } = await job.printed;
                throw new Error(`dev exited ${code} before ${count} pushes: '${error}'`);
            }
            await job.waitLines(lines);
        }
    };
    await pushed(1);
    const developing = await versions();

    // change a source file, and wait until the change is pushed
    const { files } = terminal.native.environment;
    await files.writeTextFile(join(directory, "src/note.ts"), "export const note = 1;\n");
    await pushed(2);

    // stop as interrupted, the change pushed as a new commit of the same installation on HEAD
    job.interrupt();
    const printed = await job.printed;
    const [first, second] = pushes(printed.output);
    const head = await git(directory, "rev-parse", "HEAD");
    expect({
        code: [printed.code, printed.error],
        installations: [first?.id, second?.id],
        commits: [first?.commit === head, second?.commit === head],
        parent: await git(directory, "rev-parse", `${second?.commit ?? ""}^`),
        developing,
    }).toEqual({
        code: [130, ""],
        installations: [first?.id, first?.id],
        commits: [true, false],
        parent: head,
        developing: [["reference", "."]],
    });
});

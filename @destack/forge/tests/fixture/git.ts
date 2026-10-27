import { onTestFinished } from "@destack/test";
import { join } from "node:path";
import { testHost } from "@destack/test/host";

/** The host the fixture repositories live on, which runs git. */
const host = testHost();

/** The environment of fixture git commands: no configuration, a fixed identity and time. */
const ENVIRONMENT = {
    ...host.process.variables,
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_AUTHOR_NAME: "Ada",
    GIT_AUTHOR_EMAIL: "ada@example.test",
    GIT_AUTHOR_DATE: "2026-09-27T12:00:00Z",
    GIT_COMMITTER_NAME: "Ada",
    GIT_COMMITTER_EMAIL: "ada@example.test",
    GIT_COMMITTER_DATE: "2026-09-27T12:00:00Z",
};

/** The time fixture commits and tags record, in seconds since the epoch. */
const TIME = "1790000000 +0000";

/** The object name that deletes a reference. */
const ZERO = "0".repeat(40);

/** Run git with the fixture environment and optional input, returning its trimmed output. */
export async function git(directory: string, ...arguments_: string[]): Promise<string> {
    return run(directory, arguments_, "");
}

/** Run git with input on its standard input. */
async function run(
    directory: string,
    arguments_: readonly string[],
    input: string,
): Promise<string> {
    const child = host.process.spawn(["git", "-C", directory, ...arguments_], {
        variables: ENVIRONMENT,
        stdin: "pipe",
        stdout: "pipe",
        stderr: "pipe",
    });

    // write the input and close it
    const writer = child.stdin.getWriter();
    await writer.write(new TextEncoder().encode(input));
    await writer.close();

    // read the output, the errors and the exit code
    const [output, errors, code] = await Promise.all([
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
        child.exited,
    ]);
    if (code !== 0) {
        throw new Error(`git ${arguments_.join(" ")} failed: ${errors}`);
    }

    return output.trim();
}

/** Create a temporary directory removed when the test finishes. */
export async function temporary(prefix: string): Promise<string> {
    const directory = await host.files.makeTemporaryDirectory(prefix);
    onTestFinished(() => host.files.remove(directory, { recursive: true, force: true }));

    return directory;
}

/** A developer's working tree that commits, tags and pushes to a remote. */
export class Worktree {
    /** The working tree's directory. */
    readonly directory: string;

    /** Take a working tree. */
    private constructor(directory: string) {
        this.directory = directory;
    }

    /** Start an empty working tree on main. */
    static async open(): Promise<Worktree> {
        const directory = await temporary("destack-worktree-");
        await git(directory, "init", "--quiet", "--initial-branch=main");

        return new Worktree(directory);
    }

    /** Commit one file's text, returning the commit. */
    async commit(file: string, text: string): Promise<string> {
        await host.files.writeTextFile(join(this.directory, file), text);
        await git(this.directory, "add", file);
        await git(this.directory, "commit", "--quiet", "--message", `write ${file}`);

        return git(this.directory, "rev-parse", "HEAD");
    }

    /** Tag HEAD with an annotated tag, returning the tag object. */
    async tag(name: string): Promise<string> {
        await git(this.directory, "tag", "--annotate", name, "--message", `release ${name}`);

        return git(this.directory, "rev-parse", name);
    }

    /** Push refspecs to a remote. */
    async push(remote: string, ...refspecs: string[]): Promise<void> {
        await git(this.directory, "push", "--quiet", remote, ...refspecs);
    }
}

/** History written into a bare repository in one git fast-import run. */
export class History {
    /** The fast-import commands. */
    #commands = "";
    /** The marks named so far. */
    #marks = 0;

    /** Commit one file's text onto a branch's tip, returning the commit's mark. */
    commit(branch: string, file: string, text: string): string {
        const mark = `:${++this.#marks}`;
        this.#commands += `commit refs/heads/${branch}\nmark ${mark}\ncommitter Ada <ada@example.test> ${TIME}\n${data(`write ${file}`)}M 100644 inline ${file}\n${data(text)}\n`;

        return mark;
    }

    /** Tag a marked commit with an annotated tag, returning the tag object's mark. */
    tag(name: string, commit: string): string {
        const mark = `:${++this.#marks}`;
        this.#commands += `tag ${name}\nmark ${mark}\nfrom ${commit}\ntagger Ada <ada@example.test> ${TIME}\n${data(`release ${name}`)}`;

        return mark;
    }

    /** Delete a reference. */
    delete(name: string): void {
        this.#commands += `reset ${name}\nfrom ${ZERO}\n\n`;
    }

    /** Write the history into a bare repository and return the new marks' objects. */
    async write(repository: string): Promise<string[]> {
        const marks = Array.from({ length: this.#marks }, (_, index) => `get-mark :${index + 1}\n`);
        const output = await run(
            repository,
            ["fast-import", "--quiet", "--force"],
            this.#commands + marks.join(""),
        );
        this.#commands = "";
        this.#marks = 0;

        return output === "" ? [] : output.split("\n");
    }
}

/** Frame text as a fast-import data block. */
function data(text: string): string {
    return `data ${new TextEncoder().encode(text).length}\n${text}\n`;
}

/** Serve a repository's smart HTTP reference advertisement like git http-backend. */
export async function advertise(repository: string): Promise<Uint8Array<ArrayBuffer>> {
    const child = host.process.spawn(
        ["git", "upload-pack", "--stateless-rpc", "--advertise-refs", repository],
        { variables: ENVIRONMENT, stdout: "pipe" },
    );
    const advertised = new Uint8Array(await new Response(child.stdout).arrayBuffer());
    await child.exited;
    const service = new TextEncoder().encode("001e# service=git-upload-pack\n0000");

    return new Uint8Array([...service, ...advertised]);
}

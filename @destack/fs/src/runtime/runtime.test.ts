import { chmod, mkdir, mkdtemp, rm, stat, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import { runtimeDirectory } from "./runtime.ts";

/** The process's user, which every test system has. */
const USER = process.getuid?.() ?? 0;

/** Create a temporary directory removed after the test. */
async function temporary(): Promise<string> {
    const directory = await mkdtemp(join(tmpdir(), "destack-runtime-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));

    return directory;
}

test("keep runtime files in the session's directory, or else in a per-user 0700 directory under the temporary one", async () => {
    // take the session's directory as it is
    const session = await runtimeDirectory({ environment: { XDG_RUNTIME_DIR: "/run/user/501" } });

    // create the per-user directory once and keep it on later uses
    const parent = await temporary();
    const created = await runtimeDirectory({ environment: {}, temporary: parent });
    const kept = await runtimeDirectory({ environment: {}, temporary: parent });
    const directory = join(parent, `destack-${USER}`);
    expect([session, created, kept, (await stat(directory)).mode & 0o777]).toEqual([
        "/run/user/501",
        directory,
        directory,
        0o700,
    ]);
});

test("refuse a per-user directory another user owns, the group or others may open, or a link names", async () => {
    // refuse the directory of another user's identifier
    const owned = await temporary();
    const other = USER + 1;
    expect(
        await refusal(runtimeDirectory({ environment: {}, temporary: owned, user: other })),
    ).toEqual([
        "INTERNAL_SERVER_ERROR",
        "FILE_SYSTEM_FAILED",
        `cannot trust runtime directory ${join(owned, `destack-${other}`)}: EACCES`,
    ]);

    // refuse the person's directory opened to others
    const opened = await temporary();
    await mkdir(join(opened, `destack-${USER}`), { mode: 0o700 });
    await chmod(join(opened, `destack-${USER}`), 0o755);
    expect(await refusal(runtimeDirectory({ environment: {}, temporary: opened }))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "FILE_SYSTEM_FAILED",
        `cannot trust runtime directory ${join(opened, `destack-${USER}`)}: EACCES`,
    ]);

    // refuse a link to a private directory elsewhere
    const linked = await temporary();
    await mkdir(join(linked, "elsewhere"), { mode: 0o700 });
    await symlink(join(linked, "elsewhere"), join(linked, `destack-${USER}`));
    expect(await refusal(runtimeDirectory({ environment: {}, temporary: linked }))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "FILE_SYSTEM_FAILED",
        `cannot trust runtime directory ${join(linked, `destack-${USER}`)}: EACCES`,
    ]);
});

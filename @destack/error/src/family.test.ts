import { schema } from "@destack/schema";
import { expect, test } from "vitest";
import { defineErrorFamily } from "./family.ts";
import { Failure } from "./failure.ts";

/** A note package's errors, one with typed data. */
const NoteError = defineErrorFamily(
    {
        reasons: {
            NOTE_NOT_FOUND: {},
            NOTE_BLOCKED: { data: schema.object({ holder: schema.string().min(3) }) },
        },
    },
    { package: { name: "@destack/note" } },
);

/** Another package's errors with a reason of the same name. */
const PageError = defineErrorFamily(
    { reasons: { NOTE_NOT_FOUND: {} } },
    { package: { name: "@destack/page" } },
);

test("declare a package's reasons, convert their failures to call errors, tell its failures apart and describe them for the manifest", () => {
    // fail with a reason without data and one with data
    const missing = NoteError.of("NOTE_NOT_FOUND", "note note-1 not found");
    const locked = NoteError.of("NOTE_BLOCKED", "cannot edit note 'note-1': it is locked", {
        data: { holder: "ada" },
        cause: missing,
    });

    // decode failures as a call returns them
    const decoded = Failure.error(missing.toCallError());
    const received = Failure.error(locked.toCallError());
    const foreign = Failure.error({ ...locked.toCallError(), data: { holder: 7 } });

    // convert, recognise and describe them
    expect({
        calls: [missing.toCallError(), locked.toCallError()],
        cause: locked.cause,
        is: [
            NoteError.is(missing),
            NoteError.is(missing, "NOTE_NOT_FOUND"),
            NoteError.is(missing, "NOTE_BLOCKED"),
            PageError.is(missing),
            NoteError.is(new Error("note note-1 not found")),
            NoteError.is(decoded, "NOTE_NOT_FOUND"),
            PageError.is(decoded),
            NoteError.is(received, "NOTE_BLOCKED"),
            NoteError.is(foreign, "NOTE_BLOCKED"),
        ],
        data: NoteError.is(received, "NOTE_BLOCKED") ? received.data : undefined,
        described: NoteError.describe(),
    }).toEqual({
        calls: [
            {
                code: "NOT_FOUND",
                reason: "NOTE_NOT_FOUND",
                domain: "@destack/note",
                message: "note note-1 not found",
            },
            {
                code: "PRECONDITION_FAILED",
                reason: "NOTE_BLOCKED",
                domain: "@destack/note",
                message: "cannot edit note 'note-1': it is locked",
                data: { holder: "ada" },
            },
        ],
        cause: missing,
        is: [true, true, false, false, false, true, false, true, false],
        data: { holder: "ada" },
        described: [
            { domain: "@destack/note", reason: "NOTE_NOT_FOUND", code: "NOT_FOUND" },
            { domain: "@destack/note", reason: "NOTE_BLOCKED", code: "PRECONDITION_FAILED" },
        ],
    });
});

test("refuse data that breaks a reason's schema", () => {
    // fail with a holder shorter than the schema allows
    const failed = () =>
        NoteError.of("NOTE_BLOCKED", "cannot edit note 'note-1': it is locked", {
            data: { holder: "a" },
        });

    // refuse it with the schema's one issue on the holder
    let issues: unknown;
    try {
        failed();
    } catch (error) {
        issues = error instanceof schema.Error ? error.issues : error;
    }
    expect(issues).toEqual([
        {
            origin: "string",
            code: "too_small",
            minimum: 3,
            inclusive: true,
            path: ["holder"],
            message: "Too small: expected string to have >=3 characters",
        },
    ]);
});

test("declare a family for callers by default, or for the host alone, carried by each failure", () => {
    // declare one family of each audience and fail with each
    const HostError = defineErrorFamily(
        { reasons: { DISK_BROKEN: {} }, audience: "host" },
        { package: { name: "@destack/disk" } },
    );
    const full = HostError.of("DISK_BROKEN", "the disk at /var/data is full");
    const missing = NoteError.of("NOTE_NOT_FOUND", "note note-1 not found");

    expect([NoteError.audience, missing.audience, HostError.audience, full.audience]).toEqual([
        "caller",
        "caller",
        "host",
        "host",
    ]);
});

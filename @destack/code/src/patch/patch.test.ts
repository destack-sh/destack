import { expect, test } from "@destack/test";
import { present } from "@destack/schema";
import { Source } from "../source/index.ts";
import { Random } from "../test/random.ts";
import { FilePatch, Patch } from "./index.ts";

/** A patch as git writes it: a change, a rename, a new file, a removed one, modes and a picture. */
const GIT = `diff --git a/src/plan.ts b/src/plan.ts
index 1234567..89abcde 100644
--- a/src/plan.ts
+++ b/src/plan.ts
@@ -10,3 +10,4 @@ export class Plan {
     one();
-    two();
+    two(2);
+    three();
 }
diff --git a/old name.ts b/new name.ts
similarity index 100%
rename from old name.ts
rename to new name.ts
diff --git a/new.txt b/new.txt
new file mode 100644
index 0000000..e69de29
--- /dev/null
+++ b/new.txt
@@ -0,0 +1 @@
+hi
\\ No newline at end of file
diff --git a/gone.txt b/gone.txt
deleted file mode 100755
index e69de29..0000000
diff --git a/run.sh b/run.sh
old mode 100644
new mode 100755
diff --git "a/caf\\303\\251.png" "b/caf\\303\\251.png"
index 1..2 100644
Binary files "a/caf\\303\\251.png" and "b/caf\\303\\251.png" differ
`;

test("read what git says of each file, and its lines", () => {
    const { files } = Patch.read(GIT);

    expect(
        files.map(({ status, before, after, beforeMode, afterMode, isBinary, hunks }) => [
            status,
            before,
            after,
            beforeMode,
            afterMode,
            isBinary,
            hunks.map(({ before: old, after: next, heading, lines }) => [
                `${old.start}-${old.end}:${next.start}-${next.end}`,
                heading,
                lines.map(
                    (line) => `${line.kind} ${line.text}${line.hasLineBreak ? "" : " (open)"}`,
                ),
            ]),
        ]),
    ).toEqual([
        [
            "changed",
            "src/plan.ts",
            "src/plan.ts",
            undefined,
            undefined,
            false,
            [
                [
                    "9-12:9-13",
                    "export class Plan {",
                    [
                        "same     one();",
                        "removed     two();",
                        "added     two(2);",
                        "added     three();",
                        "same }",
                    ],
                ],
            ],
        ],
        ["renamed", "old name.ts", "new name.ts", undefined, undefined, false, []],
        [
            "added",
            undefined,
            "new.txt",
            undefined,
            "100644",
            false,
            [["0-0:0-1", undefined, ["added hi (open)"]]],
        ],
        ["removed", "gone.txt", undefined, "100755", undefined, false, []],
        ["changed", "run.sh", "run.sh", "100644", "100755", false, []],
        ["changed", "café.png", "café.png", undefined, undefined, true, []],
    ]);
});

test("write, read, apply and take back the changes between any two texts", () => {
    // write random changes as git does, and read them again
    const random = new Random(3);
    const wrong = Array.from({ length: 1000 }, () => {
        const before = random.text(20);
        const after = random.changed(before);
        const patch = new Patch([FilePatch.between("a.ts", Source.of(before), Source.of(after))]);
        const read = Patch.read(patch.toString());
        const [file] = read.files;
        const [inverted] = read.invert().files;
        const isRight =
            before === after ||
            (file !== undefined &&
                inverted !== undefined &&
                JSON.stringify(read.files) === JSON.stringify(patch.files) &&
                file.apply(Source.of(before)).source.text === after &&
                inverted.apply(Source.of(after)).source.text === before);

        return isRight ? [] : [[before, after]];
    }).flat();

    expect(wrong).toEqual([]);
});

test("apply each hunk where its lines moved to, and hand back those that no longer fit", () => {
    const before = Array.from({ length: 30 }, (_, line) => `${String(line)}\n`).join("");
    const changed = before.replace("5\n", "five\n").replace("25\n", "x\n");
    const file = FilePatch.between("a", Source.of(before), Source.of(changed));
    const moved = `new\nnew\n${before.replace("25\n", "other\n")}`;
    const { source, rejected } = file.apply(Source.of(moved));
    const [first, second] = file.hunks;

    expect({
        text: source.text === moved.replace("5\n", "five\n"),
        rejected,
        left: file.without(present(first, "first hunk")).hunks,
    }).toEqual({ text: true, rejected: [second], left: [second] });
});

test("refuse a hunk whose lines do not add up", () => {
    expect(() => Patch.read("--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n one\n")).toThrow(
        new TypeError("patch is invalid: line '5' is outside its hunk"),
    );
});

import { expect, test } from "@destack/test";
import { Language } from "./index.ts";

test("find a language by a file's path or a tag, none for what is not known", () => {
    const files = ["src/plan.test.TS", "Dockerfile", "a/.zshrc", "plan.pdf", "LICENSE"];
    const tags = ["ts", "Rust", "sh", "text", ""];

    expect(files.map((file) => Language.ofFile(file)?.name)).toEqual([
        "typescript",
        "dockerfile",
        "bash",
        undefined,
        undefined,
    ]);
    expect(tags.map((tag) => Language.ofTag(tag)?.name)).toEqual([
        "typescript",
        "rust",
        "bash",
        undefined,
        undefined,
    ]);
});

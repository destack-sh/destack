import { expect, test } from "@destack/test";
import { Vocabulary } from "./vocabulary.ts";

/** The digest of the owner relation's definition. */
const OWNER = "a".repeat(64);
/** The digest of the editor relation's first definition. */
const EDITOR = "b".repeat(64);
/** The digest of another definition under the editor name. */
const WRITER = "c".repeat(64);

/** A release declaring terms with their digests. */
const release = (terms: Record<string, string>) => [{ vocabulary: terms }];

/** Plan a release against a vocabulary, reading a refusal as its message. */
function plan(vocabulary: Vocabulary, terms: Record<string, string>) {
    try {
        return Vocabulary.plan(vocabulary, release(terms)).steps;
    } catch (error) {
        if (!(error instanceof Error)) {
            throw error;
        }

        return error.message;
    }
}

test("advance a vocabulary through removal and restoration, refusing a redefined term", () => {
    const first = Vocabulary.advance(
        {},
        release({ "object/note/relation/owner": OWNER, "object/note/relation/editor": EDITOR }),
        "2026.8.0",
    );

    // remove editor in the next release
    expect(plan(first, { "object/note/relation/owner": OWNER })).toEqual([
        {
            action: "delete",
            risk: "backward-incompatible",
            target: "object/note/relation/editor",
            detail: "remove: data stored under it no longer applies",
        },
    ]);
    const second = Vocabulary.advance(
        first,
        release({ "object/note/relation/owner": OWNER }),
        "2026.9.0",
    );
    expect(second).toEqual({
        "object/note/relation/owner": { introduced: "2026.8.0", digest: OWNER },
        "object/note/relation/editor": {
            introduced: "2026.8.0",
            removed: "2026.9.0",
            digest: EDITOR,
        },
    });

    // refuse editor under another definition, which the rows of the removed one would take
    expect(
        plan(second, {
            "object/note/relation/owner": OWNER,
            "object/note/relation/editor": WRITER,
        }),
    ).toEqual(
        "object/note/relation/editor: removed in 2026.9.0 with another definition; choose a new name",
    );

    // restore editor under its last definition
    const restored = { "object/note/relation/owner": OWNER, "object/note/relation/editor": EDITOR };
    expect(plan(second, restored)).toEqual([
        {
            action: "restore",
            risk: "data-dependent",
            target: "object/note/relation/editor",
            detail: "restore after its removal in 2026.9.0: data stored under it applies again",
        },
    ]);
    expect(Vocabulary.advance(second, release(restored), "2026.10.0")).toEqual({
        "object/note/relation/owner": { introduced: "2026.8.0", digest: OWNER },
        "object/note/relation/editor": { introduced: "2026.8.0", digest: EDITOR },
    });
});

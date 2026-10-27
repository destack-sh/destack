import { Snapshot } from "@destack/db";
import { schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { ids, openSpaceDatabase } from "../../test/index.ts";
import { SelectedRepository } from "./selection.ts";

/** The repository the space's account has. */
const OWN = schema
    .identifier("repository")
    .parse("repository-01996ab0-0000-7000-8000-0000000000c1");

/** The account of another space. */
const OTHER = schema.identifier("account").parse("account-01996ab0-0000-7000-8000-0000000000c2");

test("let a space select its own account's repository, and refuse another account's as missing", async () => {
    // select one repository as owned by the space's account, then by another account
    const { database } = await openSpaceDatabase();
    const snapshot = Snapshot.live(database);
    const selection = {
        kind: "repository" as const,
        repository: OWN,
        reference: "refs/heads/main",
        directory: ".",
    };
    expect([
        await SelectedRepository.require(snapshot, ids.space, selection, ids.account),
        await refusal(SelectedRepository.require(snapshot, ids.space, selection, OTHER)),
    ]).toEqual([
        undefined,
        ["NOT_FOUND", "REPOSITORY_NOT_FOUND", `the space's account has no repository ${OWN}`],
    ]);
});

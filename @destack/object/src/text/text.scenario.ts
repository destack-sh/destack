import { defineScenario } from "@destack/package/declare";
import { objectInteraction } from "../scenario/interaction.ts";
import { trackerExample } from "../test/tracker.ts";
import { ObjectType } from "../object/object.ts";

/** The pages each text scenario writes in. */
const PAGES = 20;

/** Create the pages a text scenario writes in, unmeasured. */
const created = {
    times: PAGES,
    step: { call: "page.create", as: "alice", input: { title: "page {n}" }, bind: "pages" },
} as const;

/** Type a sentence into each page, unmeasured or measured. */
const typed = {
    call: "page.edit",
    as: "alice",
    input: {
        id: "$pages.{n}",
        field: "body",
        edits: [{ insert: "the quick brown fox", run: "a.{n}" }],
    },
} as const;

/** Type a word inside each page's sentence, after its "q". */
const inside = {
    call: "page.edit",
    as: "alice",
    input: {
        id: "$pages.{n}",
        field: "body",
        edits: [{ insert: "very ", run: "b.{n}", after: { run: "a.{n}", offset: 4 } }],
    },
} as const;

/** Type into new pages, each edit within its statements. */
export const typeIntoPages = defineScenario({
    of: ObjectType,
    interaction: objectInteraction,
    name: "type-into-pages",
    description: "type a sentence into empty pages, each edit within its statements",
    given: { examples: [trackerExample] },
    parameters: { calls: 10 },
    when: [
        created,
        {
            operation: "page.edit",
            times: "{calls}",
            warmup: "{calls}",
            step: typed,
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 19 }],
        },
    ],
    then: [{ observe: { count: "page" }, is: PAGES }],
});

/** Type inside text typed earlier, each edit within its statements. */
export const typeInsideText = defineScenario({
    of: ObjectType,
    interaction: objectInteraction,
    name: "type-inside-text",
    description: "type a word inside each page's earlier sentence, each edit within its statements",
    given: { examples: [trackerExample] },
    parameters: { calls: 10 },
    when: [
        created,
        { times: PAGES, step: typed },
        {
            operation: "page.edit",
            times: "{calls}",
            warmup: "{calls}",
            step: inside,
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 18 }],
        },
    ],
    then: [{ observe: { count: "page" }, is: PAGES }],
});

/** Delete exact pieces around a word typed inside them, each edit within its statements. */
export const deletePieces = defineScenario({
    of: ObjectType,
    interaction: objectInteraction,
    name: "delete-pieces",
    description:
        "delete each page's quick, keeping the word another person typed inside it, each edit within its statements",
    given: { examples: [trackerExample] },
    parameters: { calls: 10 },
    when: [
        created,
        { times: PAGES, step: typed },
        { times: PAGES, step: inside },
        {
            operation: "page.edit",
            times: "{calls}",
            warmup: "{calls}",
            step: {
                call: "page.edit",
                as: "alice",
                input: {
                    id: "$pages.{n}",
                    field: "body",
                    edits: [{ delete: [{ run: "a.{n}", start: 4, text: 6 }] }],
                },
            },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 15 }],
        },
    ],
    then: [{ observe: { count: "page" }, is: PAGES }],
});

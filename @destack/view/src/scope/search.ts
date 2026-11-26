import type { Localization } from "@destack/locale";
import { Choice } from "@destack/object";
import {
    type LiveQuery,
    type ObjectNode,
    Observable,
    type SearchOptions,
    type SearchResult,
} from "@destack/object/client";
import { zip } from "@destack/schema";
import { useContext } from "solid-js";
import { ActionRegistryContext, useEnvironment } from "../page/context.ts";
import { useLocale } from "../page/locale.ts";
import { debounce, leadingAndTrailing } from "../primitives/scheduled.ts";
import { type Accessor, createEffect, createSignal } from "../solid/reactive.ts";

/** How long typing pauses before a search follows it, in milliseconds: a fast typist's gap. */
const PAUSE = 150;

/** What a search finds now, which lists and palettes draw. */
export interface SearchState {
    /** The text it found the choices by: the one typed, else what that most likely meant. */
    readonly text: Accessor<string | undefined>;
    /** Whether that is another text than the one typed. */
    readonly isCorrected: Accessor<boolean>;
    /** The objects found last, undefined before any and without a text. */
    readonly choices: Accessor<readonly Choice[] | undefined>;
    /** Whether the store has yet to answer. */
    readonly isPending: Accessor<boolean>;
}

/** Search objects as a person types, of one scope or all of the view, correcting typos. */
export function useSearch(
    text: Accessor<string | undefined>,
    options: SearchOptions & {
        /** The scope the objects are in, any of the view when absent. */
        readonly scope?: string | undefined;
    },
): SearchState {
    // start with nothing found
    const actions = useContext(ActionRegistryContext);
    const locale = useLocale();
    const { clock } = useEnvironment();
    const [settled, setSettled] = createSignal<string | undefined>(undefined, { ownedWrite: true });
    const [result, setResult] = createSignal<Listing | undefined>(undefined, { ownedWrite: true });
    const [isPending, setPending] = createSignal(false, { ownedWrite: true });

    // settle the first text at once and the next once typing pauses, and clear it at once
    const settle = leadingAndTrailing(debounce, (typed: string) => setSettled(typed), PAUSE, clock);
    createEffect(text, (typed) => {
        // drop the text at once
        if (typed === undefined) {
            settle.clear();
            setSettled(undefined);
        }
        // wait out the typing
        else {
            settle(typed);
        }
    });

    // follow the settled text in the scopes it searches until either changes
    const { scope, types, ...rest } = options;
    const searched = () => {
        const typed = settled();
        const nodes = actions
            .clients()
            .filter((node) => scope === undefined || node.scope === scope);

        return typed === undefined ? undefined : { typed, nodes };
    };
    createEffect(searched, (search) => {
        // find nothing without a text
        if (search === undefined) {
            setResult(undefined);
            setPending(false);

            return undefined;
        }

        // search each scope for the types it has
        const { typed, nodes } = search;
        const searches = nodes.map((node) => {
            const held = types.filter((type) => node.objects.some((each) => each.same(type)));

            return {
                node,
                live: node.search(typed, { typos: "correct", ...rest, types: held }),
            };
        });

        // follow them, pending until the store answered
        setPending(true);
        const stopping = new AbortController();
        void Promise.all(searches.map(({ live }) => live.ready)).then(() => {
            if (!stopping.signal.aborted) {
                setPending(false);
            }
        }, reportError);
        void follow(searches, stopping.signal, (results) => {
            setResult(listing(typed, results, options.limit, locale));
        }).catch(reportError);

        return () => stopping.abort();
    });

    return {
        text: () => result()?.text,
        isCorrected: () => result()?.isCorrected ?? false,
        choices: () => result()?.choices,
        isPending,
    };
}

/** Choices, and the text they were found by. */
interface Listing {
    /** The text. */
    readonly text: string;
    /** Whether that is another text than the one typed. */
    readonly isCorrected: boolean;
    /** The choices. */
    readonly choices: readonly Choice[];
}

/** A search followed on one node. */
interface Searched {
    /** The node it runs on. */
    readonly node: ObjectNode;
    /** The search. */
    readonly live: LiveQuery<SearchResult>;
}

/** Follow searches together until the signal aborts, then close them. */
async function follow(
    searches: readonly Searched[],
    signal: AbortSignal,
    found: (results: readonly (SearchResult & Searched)[]) => void,
): Promise<void> {
    try {
        // read every search's latest result at each change
        const watched = Observable.latest(
            searches.map(({ live }) => live.watch(signal)),
            (results) =>
                zip(results, searches).map(([result, search]) => ({ ...result, ...search })),
        );
        for await (const results of watched) {
            found(results);
        }
    } finally {
        await Promise.all(searches.map(({ live }) => live.close()));
    }
}

/** List the best of every node first, of those found by what is typed when there are any. */
function listing(
    typed: string,
    results: readonly (SearchResult & Searched)[],
    limit: number,
    locale: Localization,
): Listing {
    // take the results by one text
    const found = results.filter(({ hits }) => hits.length > 0);
    const best = found.find((each) => !each.isCorrected) ?? found[0];
    const choices = found
        .filter((each) => each.text === best?.text)
        .flatMap(({ hits, node }) => hits.map((hit) => ({ hit, moment: node.context().moment })))
        .toSorted((left, right) => right.hit.relevance - left.hit.relevance)
        .slice(0, limit)
        .map(({ hit, moment }) => Choice.hit(hit, moment, locale));

    return { text: best?.text ?? typed, isCorrected: best?.isCorrected ?? false, choices };
}

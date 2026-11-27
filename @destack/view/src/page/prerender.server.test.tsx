import { testHost } from "@destack/test/host";
import { BrowserEnvironment } from "@destack/environment/browser";
import { t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { none } from "@destack/access";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { createComponent } from "solid-js";
import { renderToString } from "@solidjs/web";
import { Loading } from "../solid/flow.ts";
import { useQuery } from "../scope/query.ts";
import { EnvironmentContext, useView } from "./context.ts";

/** A note a view lists. */
const note = defineObject({
    name: "note",
    plural: "notes",
    title: t`Notes`,
    scope: Scope.universe.id,
    fields: { title: field.string() },
    access: { permissions: { read: none() } },
});

test("render a view's loading fallbacks before any host mounts it, as a prerender does", () => {
    // render a list of notes and the page's space before a mount, each under its own loading boundary
    const html = renderToString(() =>
        createComponent(EnvironmentContext, {
            value: BrowserEnvironment.server(testHost()),
            get children() {
                const notes = useQuery(note, (query) => query.findMany());

                return [
                    createComponent(Loading, {
                        fallback: "loading notes",
                        get children() {
                            return notes()
                                .map((each) => each.title)
                                .join(", ");
                        },
                    }),
                    createComponent(Loading, {
                        fallback: "loading space",
                        get children() {
                            return useView().space;
                        },
                    }),
                ];
            },
        }),
    );

    // expect both fallbacks rendered and marked as fallbacks for the browser's hydration, as a host mounts the view only there
    expect(html.replace(/<!--[^>]*-->/gu, "")).toEqual(
        'loading notesloading space<script>(self.$R=self.$R||{})[""]=[];_$HY.r["03"]="$$f";_$HY.r["04"]="$$f";</script>',
    );
});

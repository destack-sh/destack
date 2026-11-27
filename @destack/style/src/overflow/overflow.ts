import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";

/** What a box does with content past its edges, as CSS's `overflow` names it. */
export const Overflow = schema
    .enum(["visible", "hidden", "clip", "scroll", "auto"])
    .meta({ id: "style.overflow", title: "Overflow" });
/** What a box does with content past its edges. */
export type Overflow = schema.Infer<typeof Overflow>;

/** What a box does with content past its edges, titled and described. */
export const OVERFLOWS = {
    visible: { title: t.context("option")`Visible`, description: t`Shows it past the edges.` },
    hidden: {
        title: t.context("option")`Hidden`,
        description: t`Cuts it at the edges, still scrolled by code.`,
    },
    clip: { title: t.context("option")`Clip`, description: t`Cuts it at the edges.` },
    scroll: { title: t.context("option")`Scroll`, description: t`Scrolls it, with scrollbars.` },
    auto: {
        title: t.context("option")`Auto`,
        description: t`Scrolls it, with scrollbars once it runs past.`,
    },
} satisfies Record<Overflow, { readonly title: Message; readonly description: Message }>;

import { schema } from "@destack/schema";

/** A locator matching by role, with the states it requires. */
const RoleMatch = schema.object({
    /** The ARIA role, explicit or implicit, such as `tab` or `button`. */
    role: schema.string().min(1),
    /** The accessible name, matched exactly. */
    name: schema.string().exactOptional(),
    /** Whether the element is checked. */
    checked: schema.boolean().exactOptional(),
    /** Whether the element is disabled. */
    disabled: schema.boolean().exactOptional(),
    /** Whether the element is expanded. */
    expanded: schema.boolean().exactOptional(),
    /** Whether the element is pressed. */
    pressed: schema.boolean().exactOptional(),
    /** Whether the element is selected. */
    selected: schema.boolean().exactOptional(),
});

/** A locator matching a control by its label. */
const LabelMatch = schema.object({
    /** The text of the label, aria-label or aria-labelledby naming a control, matched exactly. */
    label: schema.string().min(1),
});

/** A locator matching by text. */
const TextMatch = schema.object({
    /** The whole text of the innermost element holding it, matched exactly. */
    text: schema.string().min(1),
});

/** A locator matching by test identifier. */
const TestIdMatch = schema.object({
    /** The element's `data-testid`. */
    testId: schema.string().min(1),
});

/** A locator matching by CSS selector, the last resort. */
const CssMatch = schema.object({
    /** A CSS selector. */
    css: schema.string().min(1),
});

/** What a locator matches by: a role first, CSS only as a last resort. */
export type LocatorMatch = schema.Infer<
    typeof RoleMatch | typeof LabelMatch | typeof TextMatch | typeof TestIdMatch | typeof CssMatch
>;

/** Where a step acts or an observation reads: one element, or each element for a count or texts. */
export type Locator = LocatorMatch & {
    /** The match to take by its index in document order, when several match. */
    readonly nth?: number;
    /** The element to search inside, the whole screen when absent. */
    readonly within?: Locator;
};

/** Where a locator searches and which match it takes. */
const place = {
    /** The match to take by its index in document order, when several match. */
    nth: schema.number().int().nonnegative().exactOptional(),
    /** The element to search inside, the whole screen when absent. */
    within: schema.lazy((): schema.Schema<Locator> => Locator).exactOptional(),
};

/** Where a step acts or an observation reads: by role, label, text or test identifier. */
export const Locator: schema.Schema<Locator> = schema.union([
    RoleMatch.extend(place),
    LabelMatch.extend(place),
    TextMatch.extend(place),
    TestIdMatch.extend(place),
    CssMatch.extend(place),
]);

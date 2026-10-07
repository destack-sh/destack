import { schema } from "@destack/schema";

/** What a locator matches by: a role first, CSS only as a last resort. */
export type LocatorMatch =
    | {
          /** The ARIA role, explicit or implicit, such as `tab` or `button`. */
          readonly role: string;
          /** The accessible name, matched exactly. */
          readonly name?: string;
          /** Whether the element is checked. */
          readonly checked?: boolean;
          /** Whether the element is disabled. */
          readonly disabled?: boolean;
          /** Whether the element is expanded. */
          readonly expanded?: boolean;
          /** Whether the element is pressed. */
          readonly pressed?: boolean;
          /** Whether the element is selected. */
          readonly selected?: boolean;
      }
    | {
          /** The text of the label, aria-label or aria-labelledby naming a control, matched exactly. */
          readonly label: string;
      }
    | {
          /** The whole text of the innermost element holding it, matched exactly. */
          readonly text: string;
      }
    | {
          /** The element's `data-testid`. */
          readonly testId: string;
      }
    | {
          /** A CSS selector, the last resort. */
          readonly css: string;
      };

/** Where a step acts or an observation reads: one element, or each element for a count or texts. */
export type Locator = LocatorMatch & {
    /** The match to take by its index in document order, when several match. */
    readonly nth?: number;
    /** The element to search inside, the whole screen when absent. */
    readonly within?: Locator;
};

/** The schema of where a locator searches and which match it takes. */
const place = {
    /** The match to take by its index in document order, when several match. */
    nth: schema.number().int().nonnegative().exactOptional(),
    /** The element to search inside, the whole screen when absent. */
    within: schema.lazy((): schema.Schema<Locator> => Locator).exactOptional(),
};

/** Where a step acts or an observation reads: by role, label, text or test identifier. */
export const Locator: schema.Schema<Locator> = schema.union([
    schema.object({
        /** The ARIA role. */
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
        ...place,
    }),
    schema.object({
        /** The label text. */
        label: schema.string().min(1),
        ...place,
    }),
    schema.object({
        /** The whole text. */
        text: schema.string().min(1),
        ...place,
    }),
    schema.object({
        /** The test id. */
        testId: schema.string().min(1),
        ...place,
    }),
    schema.object({
        /** The CSS selector. */
        css: schema.string().min(1),
        ...place,
    }),
]);

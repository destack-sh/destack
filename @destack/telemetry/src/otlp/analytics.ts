/** The event name of a log record recording a view a person opened. */
export const VISIT_EVENT = "destack.visit";

/** The event name of a log record recording an action a person took. */
export const ACTION_EVENT = "destack.action";

/** The prefix of the event names the platform reserves for its own records. */
export const RESERVED_EVENT_PREFIX = "destack.";

/** The attributes of visit and action records, by what they carry. */
export const ANALYTICS_ATTRIBUTES = {
    /** The view's route pattern, as the semantic conventions name a URL template. */
    route: "url.template",
    /** The path opened. */
    path: "url.path",
    /** The page's title. */
    title: "destack.title",
    /** The host the visitor came from. */
    referrer: "destack.referrer",
    /** The visitor's locale. */
    locale: "destack.locale",
    /** The action's name. */
    action: "destack.action.name",
} as const;

/** What a view records of a visit beside its route. */
export interface VisitDetail {
    /** The path opened. */
    readonly path?: string;
    /** The page's title. */
    readonly title?: string;
    /** The host the visitor came from. */
    readonly referrer?: string;
    /** The visitor's locale. */
    readonly locale?: string;
}

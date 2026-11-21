import type { BrowserContext, CDPSession, Page } from "playwright";

/** The moves a pointer makes on its way to a point. */
const MOVE_STEPS = 4;

/** The pressed buttons the protocol reports for each button, as a bit mask. */
const BUTTONS = { none: 0, left: 1, right: 2 } as const;

/** A point in the page's viewport, in CSS pixels. */
export interface ViewportPoint {
    /** The horizontal position. */
    readonly x: number;
    /** The vertical position. */
    readonly y: number;
}

/** The button of a pointer a step presses. */
export type PointerButton = "primary" | "secondary";

/** The device a browser driver points with: it moves, presses and releases as that device does. */
export interface BrowserPointer {
    /** Move from one point to another in steps. */
    readonly move: (from: ViewportPoint, to: ViewportPoint) => Promise<void>;
    /** Press a button at a point. */
    readonly down: (at: ViewportPoint, button: PointerButton) => Promise<void>;
    /** Release the button it pressed, at a point. */
    readonly up: (at: ViewportPoint) => Promise<void>;
}

/** Open the pointer a scenario's globals ask for on a page, a mouse when they ask for none. */
export async function openPointer(
    kind: "mouse" | "pen" | "touch",
    context: BrowserContext,
    page: Page,
): Promise<BrowserPointer> {
    // a finger or a pen sends its input through the Chrome DevTools Protocol
    if (kind === "touch") {
        return new TouchPointer(await context.newCDPSession(page));
    }
    // a pen
    else if (kind === "pen") {
        return new PenPointer(await context.newCDPSession(page));
    }

    return new MousePointer(page);
}

/** A mouse, through Playwright's mouse. */
class MousePointer implements BrowserPointer {
    /** The page it points at. */
    readonly #page: Page;
    /** The button it pressed last. */
    #button: "left" | "right" = "left";

    /** Create a mouse on a page. */
    constructor(page: Page) {
        this.#page = page;
    }

    /** Move the mouse to a point in steps. */
    async move(_from: ViewportPoint, to: ViewportPoint): Promise<void> {
        await this.#page.mouse.move(to.x, to.y, { steps: MOVE_STEPS });
    }

    /** Press a mouse button. */
    async down(_at: ViewportPoint, button: PointerButton): Promise<void> {
        this.#button = buttonOf(button);
        await this.#page.mouse.down({ button: this.#button });
    }

    /** Release the mouse button it pressed. */
    async up(): Promise<void> {
        await this.#page.mouse.up({ button: this.#button });
    }
}

/** A finger, through the protocol's touch events. */
class TouchPointer implements BrowserPointer {
    /** The session that sends the touches. */
    readonly #session: CDPSession;
    /** Whether the finger is on the screen, which a move needs to send anything. */
    #isDown = false;

    /** Create a finger on a session. */
    constructor(session: CDPSession) {
        this.#session = session;
    }

    /** Slide a finger that is down to a point, and send nothing for one that hovers. */
    async move(from: ViewportPoint, to: ViewportPoint): Promise<void> {
        if (!this.#isDown) {
            return;
        }
        for (const point of stepsBetween(from, to)) {
            await this.#session.send("Input.dispatchTouchEvent", {
                type: "touchMove",
                touchPoints: [{ ...point, id: 1 }],
            });
        }
    }

    /** Land the finger at a point. */
    async down(at: ViewportPoint): Promise<void> {
        this.#isDown = true;
        await this.#session.send("Input.dispatchTouchEvent", {
            type: "touchStart",
            touchPoints: [{ ...at, id: 1 }],
        });
    }

    /** Lift the finger. */
    async up(): Promise<void> {
        this.#isDown = false;
        await this.#session.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    }
}

/** A pen, through the protocol's mouse events marked as a pen's. */
class PenPointer implements BrowserPointer {
    /** The session that sends the pen's input. */
    readonly #session: CDPSession;
    /** The button the pen presses, none while it hovers. */
    #button: "left" | "right" | "none" = "none";

    /** Create a pen on a session. */
    constructor(session: CDPSession) {
        this.#session = session;
    }

    /** Move the pen to a point in steps. */
    async move(from: ViewportPoint, to: ViewportPoint): Promise<void> {
        for (const point of stepsBetween(from, to)) {
            await this.#send("mouseMoved", point);
        }
    }

    /** Press the pen at a point. */
    async down(at: ViewportPoint, button: PointerButton): Promise<void> {
        this.#button = buttonOf(button);
        await this.#send("mousePressed", at);
    }

    /** Lift the pen at a point. */
    async up(at: ViewportPoint): Promise<void> {
        await this.#send("mouseReleased", at);
        this.#button = "none";
    }

    /** Send a move, a press or a release of the pen at a point. */
    async #send(
        type: "mouseMoved" | "mousePressed" | "mouseReleased",
        point: ViewportPoint,
    ): Promise<void> {
        await this.#session.send("Input.dispatchMouseEvent", {
            type,
            ...point,
            button: this.#button,
            buttons: BUTTONS[this.#button],
            clickCount: type === "mouseMoved" ? 0 : 1,
            pointerType: "pen",
        });
    }
}

/** Read the browser's name of a step's button. */
function buttonOf(button: PointerButton): "left" | "right" {
    return button === "secondary" ? "right" : "left";
}

/** List the points a move passes through on its way, the last one its end. */
function stepsBetween(from: ViewportPoint, to: ViewportPoint): ViewportPoint[] {
    return Array.from({ length: MOVE_STEPS }, (_, index) => {
        const share = (index + 1) / MOVE_STEPS;

        return { x: from.x + (to.x - from.x) * share, y: from.y + (to.y - from.y) * share };
    });
}

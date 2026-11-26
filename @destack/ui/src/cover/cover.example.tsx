import { t } from "@destack/locale";
import arrowClockwise from "@destack/icon/phosphor/arrow-clockwise";
import compass from "@destack/icon/phosphor/compass";
import house from "@destack/icon/phosphor/house";
import lockSimple from "@destack/icon/phosphor/lock-simple";
import warningCircle from "@destack/icon/phosphor/warning-circle";
import wifiSlash from "@destack/icon/phosphor/wifi-slash";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, font, space, weight } from "@destack/theme/tokens.stylex";
import { Button } from "../button/index.ts";
import { Cluster } from "../cluster/index.ts";
import { DataList, DataListItem, DataListLabel, DataListValue } from "../data-list/index.ts";
import {
    Empty,
    EmptyContent,
    EmptyDescription,
    EmptyHeader,
    EmptyMedia,
    EmptyTitle,
} from "../empty/index.ts";
import { Icon } from "../icon/index.ts";
import { Cover, CoverCenter } from "./cover.tsx";

/** The styles of the welcome screen's text and actions. */
const styles = style.create({
    brand: {
        margin: 0,
        padding: space[5],
        color: color.mutedForeground,
        fontWeight: weight.medium,
    },
    title: {
        margin: 0,
        textAlign: "center",
    },
    actions: {
        display: "flex",
        justifyContent: "center",
        padding: space[6],
    },
    details: {
        textAlign: "start",
    },
    code: {
        fontFamily: font.code,
    },
});

/** A welcome screen with its heading centered and its actions at the bottom. */
export const coverWelcome = defineExample({
    of: Cover,
    name: "welcome",
    description: t`A welcome screen with its heading centered and its actions at the bottom.`,
    layout: "fullscreen",
    properties: {
        minHeight: "32rem",
    },
    render: (properties) => (
        <Cover {...properties}>
            <p {...style.attrs(text.sm, styles.brand)}>Destack</p>
            <CoverCenter
                render={(attributes) => <h1 {...attributes} />}
                xstyle={[text["4xl"], styles.title]}
            >
                Take back your software
            </CoverCenter>
            <div {...style.attrs(styles.actions)}>
                <Button size="lg">Get started</Button>
            </div>
        </Cover>
    ),
});

/** A page that was not found, with the way back home. */
export const coverNotFound = defineExample({
    of: Cover,
    name: "notFound",
    description: t`A page that was not found, with the way back home.`,
    layout: "fullscreen",
    properties: {
        minHeight: "28rem",
    },
    render: (properties) => (
        <Cover {...properties}>
            <CoverCenter>
                <Empty>
                    <EmptyHeader>
                        <EmptyMedia variant="icon">
                            <Icon icon={compass} />
                        </EmptyMedia>
                        <EmptyTitle>Page not found</EmptyTitle>
                        <EmptyDescription>
                            Its owner moved or deleted it, or the link has a typo.
                        </EmptyDescription>
                    </EmptyHeader>
                    <EmptyContent>
                        <Button variant="outline">
                            <Icon icon={house} />
                            Go home
                        </Button>
                    </EmptyContent>
                </Empty>
            </CoverCenter>
        </Cover>
    ),
});

/** A page the person may not open, with a way to sign in as someone else. */
export const coverForbidden = defineExample({
    of: Cover,
    name: "forbidden",
    description: t`A page the person may not open, with a way to sign in as someone else.`,
    layout: "fullscreen",
    properties: {
        minHeight: "28rem",
    },
    render: (properties) => (
        <Cover {...properties}>
            <CoverCenter>
                <Empty>
                    <EmptyHeader>
                        <EmptyMedia variant="icon">
                            <Icon icon={lockSimple} />
                        </EmptyMedia>
                        <EmptyTitle>You can't view this</EmptyTitle>
                        <EmptyDescription>
                            Ask its owner for access, or sign in with another account.
                        </EmptyDescription>
                    </EmptyHeader>
                    <EmptyContent>
                        <Cluster justify="center">
                            <Button>Sign in</Button>
                            <Button variant="outline">Go home</Button>
                        </Cluster>
                    </EmptyContent>
                </Empty>
            </CoverCenter>
        </Cover>
    ),
});

/** A page that failed, with its code and request id to report and a way to try again. */
export const coverError = defineExample({
    of: Cover,
    name: "error",
    description: t`A page that failed, with its code and request id to report and a way to try again.`,
    layout: "fullscreen",
    properties: {
        minHeight: "32rem",
    },
    render: (properties) => (
        <Cover {...properties}>
            <CoverCenter>
                <Empty>
                    <EmptyHeader>
                        <EmptyMedia variant="icon">
                            <Icon icon={warningCircle} />
                        </EmptyMedia>
                        <EmptyTitle>Something went wrong</EmptyTitle>
                        <EmptyDescription>
                            The page failed to load. Try again, or send these details to support.
                        </EmptyDescription>
                    </EmptyHeader>
                    <DataList xstyle={styles.details}>
                        <DataListItem>
                            <DataListLabel>Code</DataListLabel>
                            <DataListValue xstyle={styles.code}>500</DataListValue>
                        </DataListItem>
                        <DataListItem>
                            <DataListLabel>Request</DataListLabel>
                            <DataListValue xstyle={styles.code}>req_7f3a9c2e41b8</DataListValue>
                        </DataListItem>
                    </DataList>
                    <EmptyContent>
                        <Cluster justify="center">
                            <Button>
                                <Icon icon={arrowClockwise} />
                                Try again
                            </Button>
                            <Button variant="outline">Go home</Button>
                        </Cluster>
                    </EmptyContent>
                </Empty>
            </CoverCenter>
        </Cover>
    ),
});

/** A page that cannot load while the device is offline, retrying once it reconnects. */
export const coverOffline = defineExample({
    of: Cover,
    name: "offline",
    description: t`A page that cannot load while the device is offline, retrying once it reconnects.`,
    layout: "fullscreen",
    properties: {
        minHeight: "28rem",
    },
    render: (properties) => (
        <Cover {...properties}>
            <CoverCenter>
                <Empty>
                    <EmptyHeader>
                        <EmptyMedia variant="icon">
                            <Icon icon={wifiSlash} />
                        </EmptyMedia>
                        <EmptyTitle>You're offline</EmptyTitle>
                        <EmptyDescription>
                            This page loads again once your connection is back.
                        </EmptyDescription>
                    </EmptyHeader>
                    <EmptyContent>
                        <Button variant="outline">
                            <Icon icon={arrowClockwise} />
                            Try now
                        </Button>
                    </EmptyContent>
                </Empty>
            </CoverCenter>
        </Cover>
    ),
});

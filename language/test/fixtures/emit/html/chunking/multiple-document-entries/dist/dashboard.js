const __destack_resource_cd6d2170 = "./dashboard-shell.css";
if(typeof document !== "undefined") {
    const __destack_stylesheet_link_cd6d2170 = document.createElement("link");
    __destack_stylesheet_link_cd6d2170.rel="stylesheet";
    __destack_stylesheet_link_cd6d2170.href=__destack_resource_cd6d2170;
    document.head.appendChild(__destack_stylesheet_link_cd6d2170);
}
export { __destack_resource_cd6d2170 as default };

import {
    getNavigationLabel,
    getPageDescription,
    getPageHeading,
    getPageTitle,
} from "./page-metadata.js";
export const dashboardPageState = {
    pageTitle: getPageTitle("configuration"),
    pageHeading: getPageHeading("configuration"),
    pageDescription: getPageDescription("configuration"),
    navigationLabel: getNavigationLabel("configuration"),
};
//# sourceMappingURL=./dashboard.js.map

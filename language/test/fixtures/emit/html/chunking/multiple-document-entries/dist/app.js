const __destack_resource_d90f1d3d = "./shell.css";
if(typeof document !== "undefined") {
    const __destack_stylesheet_link_d90f1d3d = document.createElement("link");
    __destack_stylesheet_link_d90f1d3d.rel="stylesheet";
    __destack_stylesheet_link_d90f1d3d.href=__destack_resource_d90f1d3d;
    document.head.appendChild(__destack_stylesheet_link_d90f1d3d);
}
export { __destack_resource_d90f1d3d as default };

import {
    getNavigationLabel,
    getPageDescription,
    getPageHeading,
    getPageTitle,
} from "./page-metadata.js";
export const appPageState = {
    pageTitle: getPageTitle("github"),
    pageHeading: getPageHeading("github"),
    pageDescription: getPageDescription("github"),
    navigationLabel: getNavigationLabel("github"),
};
//# sourceMappingURL=./app.js.map

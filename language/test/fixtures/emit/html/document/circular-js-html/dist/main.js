const __destack_resource_0897065c = "<!DOCTYPE html>\n<html>\n  <head>\n    <link rel=\"stylesheet\" href=\"./page.css\">\n    <script src=\"./bootstrap.ts\"></script>\n    <link rel=\"modulepreload\" href=\"./main.ts\">\n  </head>\n  <body>\n    <div id=\"content\">Circular Import Test</div>\n  </body>\n</html>\n";

export function formatBootstrapMode(mode) {
    return `bootstrap:${mode}`;
}

export function buildPageMessage(label) {
    return `page:${label}`;
}

const page = __destack_resource_0897065c;
const pageMessage = buildPageMessage("circular-import");
console.log("Main JS loaded page:", page, pageMessage, formatBootstrapMode("module"));
//# sourceMappingURL=./main.js.map

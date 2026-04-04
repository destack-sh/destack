const __destack_resource_5858ca82 = "<!DOCTYPE html>\n<html>\n  <head>\n    <link rel=\"stylesheet\" href=\"./about.css\">\n    <script src=\"./about.ts\"></script>\n  </head>\n  <body>\n    <h1>About Page</h1>\n  </body>\n</html>\n";

const __destack_resource_aedb55ac = "<!DOCTYPE html>\n<html>\n  <head>\n    <link rel=\"stylesheet\" href=\"./home.css\">\n    <script src=\"./home.ts\"></script>\n  </head>\n  <body>\n    <h1>Home Page</h1>\n  </body>\n</html>\n";

const homeHtml = __destack_resource_aedb55ac;
const aboutHtml = __destack_resource_5858ca82;
if(typeof homeHtml !== "string") {
    throw new Error("Expected homeHtml to be an HTML string");
}
if(typeof aboutHtml !== "string") {
    throw new Error("Expected aboutHtml to be an HTML string");
}
console.log("Home HTML:", homeHtml);
console.log("About HTML:", aboutHtml);
//# sourceMappingURL=./server.js.map

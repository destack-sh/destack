import htmlUrl from "./page.html" with { type: "file" };
import htmlDocument from "./index.html";

if (typeof htmlUrl !== "string") {
  throw new Error("Expected htmlUrl to be a string, got " + typeof htmlUrl);
}

if (typeof htmlDocument !== "string" || !htmlDocument.includes("<h1>Test Page</h1>")) {
  throw new Error("Expected htmlDocument to be an HTML string");
}

console.log("✓ File import returned URL:", htmlUrl);
console.log("✓ HTML import returned text:", htmlDocument);
console.log("✓ Both import types work correctly");

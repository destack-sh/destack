import htmlUrl from "./page.html" with { type: "file" };
import htmlManifest from "./index.html";

if (typeof htmlUrl !== "string") {
  throw new Error("Expected htmlUrl to be a string, got " + typeof htmlUrl);
}

if (typeof htmlManifest !== "object" || !htmlManifest.index || !Array.isArray(htmlManifest.files)) {
  throw new Error("Expected htmlManifest to be an object with index and files array");
}

console.log("✓ File import returned URL:", htmlUrl);
console.log("✓ HTML import returned manifest with", htmlManifest.files.length, "files");
console.log("✓ Both import types work correctly");

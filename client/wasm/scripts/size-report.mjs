#!/usr/bin/env bun

import { brotliCompressSync, gzipSync } from "node:zlib";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

const DECIMAL_PLACES = 2;
const includeBrotli = process.argv.includes("--brotli");
const dist = fileURLToPath(new URL("../dist", import.meta.url));

function bytes(value) {
    const units = ["B", "KiB", "MiB", "GiB"];
    let size = value;
    let index = 0;

    // scale to the largest readable unit
    while (size >= 1024 && index < units.length - 1) {
        size /= 1024;
        index += 1;
    }

    return `${size.toFixed(index === 0 ? 0 : DECIMAL_PLACES)} ${units[index]}`;
}

function reportFile(name) {
    const path = join(dist, name);
    const content = readFileSync(path);

    return {
        name,
        raw: content.byteLength,
        gzip: gzipSync(content).byteLength,
        brotli: includeBrotli ? brotliCompressSync(content).byteLength : undefined,
    };
}

// collect files from the published package directory
const files = readdirSync(dist).filter((name) => statSync(join(dist, name)).isFile());
const reports = files.map(reportFile).sort((left, right) => right.raw - left.raw);

// add independent transfer sizes
const total = reports.reduce(
    (size, file) => ({
        raw: size.raw + file.raw,
        gzip: size.gzip + file.gzip,
        brotli: includeBrotli ? size.brotli + file.brotli : undefined,
    }),
    { raw: 0, gzip: 0, brotli: includeBrotli ? 0 : undefined },
);

console.log("Destack WASM package size");
console.log("");
console.log(`raw:    ${bytes(total.raw)}`);
console.log(`gzip:   ${bytes(total.gzip)}`);

if (includeBrotli) {
    console.log(`brotli: ${bytes(total.brotli)}`);
}

console.log("");
console.log("Largest files");

for (const file of reports.slice(0, 8)) {
    let row = `${file.name.padEnd(30)} ${bytes(file.raw).padStart(10)} ${bytes(file.gzip).padStart(10)}`;
    row = includeBrotli ? `${row} ${bytes(file.brotli).padStart(10)}` : row;

    console.log(row);
}

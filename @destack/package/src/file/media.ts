/** The media types of files by extension (nginx's mime.types). */
const MEDIA_TYPES: Readonly<Record<string, string>> = {
    // documents and scripts
    html: "text/html",
    htm: "text/html",
    xhtml: "application/xhtml+xml",
    css: "text/css",
    js: "text/javascript",
    mjs: "text/javascript",
    cjs: "text/javascript",
    wasm: "application/wasm",

    // data and text
    json: "application/json",
    map: "application/json",
    jsonld: "application/ld+json",
    webmanifest: "application/manifest+json",
    xml: "application/xml",
    atom: "application/atom+xml",
    rss: "application/rss+xml",
    txt: "text/plain",
    md: "text/markdown",
    csv: "text/csv",
    ics: "text/calendar",
    vtt: "text/vtt",

    // images
    png: "image/png",
    apng: "image/apng",
    jpg: "image/jpeg",
    jpeg: "image/jpeg",
    gif: "image/gif",
    webp: "image/webp",
    avif: "image/avif",
    svg: "image/svg+xml",
    ico: "image/vnd.microsoft.icon",
    bmp: "image/bmp",
    tif: "image/tiff",
    tiff: "image/tiff",

    // fonts
    woff: "font/woff",
    woff2: "font/woff2",
    ttf: "font/ttf",
    otf: "font/otf",

    // audio
    mp3: "audio/mpeg",
    m4a: "audio/mp4",
    aac: "audio/aac",
    wav: "audio/wav",
    ogg: "audio/ogg",
    oga: "audio/ogg",
    opus: "audio/opus",
    flac: "audio/flac",
    weba: "audio/webm",

    // video and streams
    mp4: "video/mp4",
    m4v: "video/mp4",
    webm: "video/webm",
    ogv: "video/ogg",
    mov: "video/quicktime",
    m3u8: "application/vnd.apple.mpegurl",

    // models
    glb: "model/gltf-binary",
    gltf: "model/gltf+json",

    // files people download
    pdf: "application/pdf",
    epub: "application/epub+zip",
    zip: "application/zip",
    gz: "application/gzip",
    tar: "application/x-tar",
    rtf: "application/rtf",
    docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    pptx: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    odt: "application/vnd.oasis.opendocument.text",
    ods: "application/vnd.oasis.opendocument.spreadsheet",
    odp: "application/vnd.oasis.opendocument.presentation",
};

/** The media type of a file whose extension tells none (RFC 2046 4.5.1). */
const UNKNOWN = "application/octet-stream";

/** Media types of files. */
export const MediaType = {
    /** Read a file's media type by its extension. */
    of(path: string): string {
        return MEDIA_TYPES[path.slice(path.lastIndexOf(".") + 1).toLowerCase()] ?? UNKNOWN;
    },
};

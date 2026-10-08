# @destack/web

`WebOptions` is SolidStart's app config (`app`, `ssr`, `prerender.routes`, `entry-server.tsx` and `entry-client.tsx`) as a build output, and `metadata` writes Next.js's metadata files (`sitemap.xml`, `robots.txt`, `manifest.webmanifest`).

```ts
const website: WebOptions = {
    kind: "web",
    app: "src/app.tsx",
    ssr: { runtime: "bun" },
    site: "https://example.com",
    prerender: { routes: ["/"], notFound: "/404" },
    metadata: { sitemap: ["/"], robots: { rules: [{ userAgent: "*", allow: ["/"] }] } }, // Next's sitemap.ts and robots.ts as options
};

await using build = await buildPackage({ directory, dependencies, outputs: { website } });
```

## Site files

`metadata` also writes an Atom feed, `llms.txt` and `/.well-known/security.txt`.

```ts
metadata: {
    feed: { path: "/blog/feed.xml", home: "/blog/", title: "Blog", author: { name: "Ada" }, entries },
    llms: { title: "Example", summary: "What the site is.", sections: [] },
    security: { contact: ["mailto:security@example.com"], expires: "2027-04-01T00:00:00Z" },
}
```

## Development server

`DevelopmentServer.start` serves a web application with Vite, restarting when a `destack.json` changes.

```ts
await using server = await DevelopmentServer.start({
    directory,
    application: website,
    server: { port: 3000 },
});
```

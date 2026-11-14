// Run one query against the demo's Pagefind index, the way the search page does.
//
//   node scripts/pagefind-query.mjs http://localhost:3000 "Codemotion Roma 2026"
//
// Prints one line per hit, `title<TAB>url`, best first, and exits 0 even when
// nothing matched: an empty answer is a result, and scripts/verify-demo.sh
// decides what it means.
//
// WHY THIS EXISTS. /search renders server results and then hands the page to
// the kernel's scolta.js, which asks the static Pagefind index and shows only
// what that says. So whether a visitor finds a conference is a question about
// the index, not about the HTML the server sent, and the only faithful way to
// ask it is to load the same pagefind.js the page loads and call `search()`.
// Grepping the index's fragments, which is what verify-demo.sh did before,
// passed while every title search in a real browser answered "No results
// found" (FRICTION.md, G-PAGEFIND-INDEX-OMITS-TITLES).
//
// pagefind.js is a browser module. It is fetched from the running demo, never
// from anywhere else, written to a private temporary directory and imported
// from there, with the four browser globals it reads stubbed: `window` and
// `location` for its base path, `document` for the page language, and `fetch`
// made absolute, because the module strips the origin off its own base path
// and Node cannot fetch a relative URL.

import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [base, query] = process.argv.slice(2);
if (!base || !query) {
    console.error("usage: node scripts/pagefind-query.mjs <base-url> <query>");
    process.exit(2);
}

const response = await fetch(`${base}/static/pagefind/pagefind.js`);
if (!response.ok) {
    console.error(`pagefind.js answered ${response.status}`);
    process.exit(1);
}

const dir = await mkdtemp(join(tmpdir(), "ritrovo-pagefind-"));
try {
    const file = join(dir, "pagefind.mjs");
    await writeFile(file, await response.text());

    globalThis.window = { location: { origin: base, href: `${base}/` } };
    globalThis.location = globalThis.window.location;
    globalThis.document = {
        currentScript: null,
        querySelector: () => ({ getAttribute: () => "en" }),
    };
    const realFetch = globalThis.fetch;
    globalThis.fetch = (url, options) =>
        realFetch(typeof url === "string" && url.startsWith("/") ? base + url : url, options);

    const pagefind = await import(file);
    await pagefind.options({ basePath: `${base}/static/pagefind/` });
    const found = await pagefind.search(query);
    for (const hit of found.results.slice(0, 20)) {
        const data = await hit.data();
        // scolta.js links a result to meta.url, which the kernel sets to the
        // item's alias or /item/{id}; data.url is the fragment's file name.
        console.log(`${data.meta?.title ?? ""}\t${data.meta?.url ?? data.url}`);
    }
} finally {
    await rm(dir, { recursive: true, force: true });
}

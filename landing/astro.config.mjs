// @ts-check
import { defineConfig } from "astro/config";

// Static output (the default) — deployed to Cloudflare Pages. No SSR/adapter needed: the dial and
// theme switcher are tiny client islands, everything else is pre-rendered HTML.
export default defineConfig({
  site: "https://usageos.app",
});

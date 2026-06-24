/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx,js,jsx}"],
  theme: {
    extend: {
      // Colours are driven entirely by the design tokens in src/styles/tokens.css,
      // swapped per [data-theme]. Components reference these (bg-bg, text-fg,
      // border-edge, text-c-deep, …); never a literal colour.
      colors: {
        bg: "var(--bg)",
        surface: "var(--surface)",
        fg: "var(--fg)",
        muted: "var(--muted)",
        edge: "var(--edge)",
        rule: "var(--rule)",
        track: "var(--track)",
        "c-deep": "var(--c-deep)",
        "c-research": "var(--c-research)",
        "c-comms": "var(--c-comms)",
        "c-breaks": "var(--c-breaks)",
        now: "var(--now)",
        casing: "var(--casing)",
        "on-ink": "var(--on-ink)",
        "bar-bg": "var(--bar-bg)",
        "bar-fg": "var(--bar-fg)",
      },
      fontFamily: {
        // Anton = poster display (all-caps); Jost = everything else.
        display: ["Anton", "sans-serif"],
        sans: ["Jost", "system-ui", "sans-serif"],
      },
      borderRadius: {
        // Hard edges everywhere; the only radius is the app window frame.
        frame: "5px",
      },
      keyframes: {
        pulse: {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.35" },
        },
      },
      animation: {
        // The "Tracking" status dot.
        pulse: "pulse 2.4s ease-in-out infinite",
      },
    },
  },
  plugins: [],
};

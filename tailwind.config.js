/** @type {import('tailwindcss').Config} */

// Colors that need alpha modifiers (bg-accent/15) are stored as RGB triplets in
// CSS variables (see src/styles.css) so one definition serves light and dark.
const rgb = (name) => `rgb(var(--c-${name}) / <alpha-value>)`;
const label = (alpha) => `rgb(var(--c-label) / ${alpha})`;

export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: "media",
  theme: {
    // The palette is closed on purpose: every color in the UI comes from the
    // macOS-style tokens below, never from stock Tailwind grays/blues.
    colors: {
      transparent: "transparent",
      current: "currentColor",
      white: "#ffffff",
      black: "#000000",

      accent: rgb("accent"),
      "accent-fg": rgb("accent-fg"),
      blue: rgb("blue"),
      purple: rgb("purple"),
      green: rgb("green"),
      red: rgb("red"),
      orange: rgb("orange"),
      yellow: rgb("yellow"),
      pink: rgb("pink"),
      teal: rgb("teal"),
      indigo: rgb("indigo"),
      "purple-fg": rgb("purple-fg"),
      "green-fg": rgb("green-fg"),
      "red-fg": rgb("red-fg"),
      "orange-fg": rgb("orange-fg"),
      "yellow-fg": rgb("yellow-fg"),

      // Label hierarchy (primary → quaternary), like NSColor.labelColor & friends.
      label: label(0.86),
      "label-2": label(0.58),
      "label-3": label(0.38),
      "label-4": label(0.16),

      // Translucent fills & surfaces that sit on top of the glass panel.
      fill: "var(--fill-1)",
      "fill-2": "var(--fill-2)",
      "fill-3": "var(--fill-3)",
      separator: "var(--separator)",
      surface: "var(--surface)",
      "surface-2": "var(--surface-2)",
    },
    fontFamily: {
      sans: [
        "-apple-system",
        "BlinkMacSystemFont",
        '"SF Pro Text"',
        '"Apple SD Gothic Neo"',
        "system-ui",
        "sans-serif",
      ],
      mono: ["ui-monospace", '"SF Mono"', "SFMono-Regular", "Menlo", "monospace"],
    },
    fontSize: {
      // macOS type scale: 13pt is the system body size.
      title: ["15px", { lineHeight: "20px", fontWeight: "600", letterSpacing: "-0.012em" }],
      headline: ["13px", { lineHeight: "18px", fontWeight: "600" }],
      body: ["13px", { lineHeight: "18px" }],
      // Longer reading/writing surfaces get more leading — Hangul needs it.
      reading: ["14px", { lineHeight: "23px" }],
      sub: ["12px", { lineHeight: "16px" }],
      caption: ["11px", { lineHeight: "14px" }],
      micro: ["10px", { lineHeight: "12px" }],
    },
    extend: {
      borderRadius: {
        panel: "20px",
        card: "14px",
        pane: "12px",
        control: "9px",
      },
      boxShadow: {
        note: "var(--shadow-note)",
        "note-hover": "var(--shadow-note-hover)",
        pop: "var(--shadow-pop)",
        sheet: "var(--shadow-sheet)",
      },
      transitionTimingFunction: {
        out: "cubic-bezier(0.22, 1, 0.36, 1)",
        spring: "cubic-bezier(0.34, 1.4, 0.64, 1)",
      },
      keyframes: {
        "panel-in": {
          from: { opacity: "0", transform: "translateY(8px) scale(0.99)" },
          to: { opacity: "1", transform: "none" },
        },
        "pop-in": {
          from: { opacity: "0", transform: "translateY(4px) scale(0.97)" },
          to: { opacity: "1", transform: "none" },
        },
        "fade-in": {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
        spinner: {
          to: { transform: "rotate(360deg)" },
        },
        caret: {
          "0%, 49%": { opacity: "1" },
          "50%, 100%": { opacity: "0" },
        },
      },
      animation: {
        "panel-in": "panel-in 240ms cubic-bezier(0.22, 1, 0.36, 1) both",
        "pop-in": "pop-in 180ms cubic-bezier(0.22, 1, 0.36, 1) both",
        "fade-in": "fade-in 160ms ease-out both",
        spinner: "spinner 0.9s steps(12) infinite",
        caret: "caret 1.05s step-end infinite",
      },
    },
  },
  plugins: [],
};

// Color themes, type themes and light/dark mode.
//
// A color theme is a set of design tokens with a light and a dark variant. A type theme is a
// pair of fonts bundled with the app. The tokens become CSS custom properties on the root
// element, so every component follows them, and the reader passes them on to PDF.js.

export type Appearance = "system" | "light" | "dark";
export type Mode = "light" | "dark";

export interface Palette {
  bg: string;
  panel: string;
  raised: string;
  fg: string;
  muted: string;
  line: string;
  accent: string;
  accentFg: string;
  danger: string;
}

export interface ColorTheme {
  id: string;
  name: string;
  description: string;
  light: Palette;
  dark: Palette;
}

export interface TypeTheme {
  id: string;
  name: string;
  description: string;
  /** Interface text. */
  ui: string;
  /** Titles and headings. */
  display: string;
  /** Text on placeholder covers. */
  cover: string;
}

export const colorThemes: ColorTheme[] = [
  {
    id: "oak",
    name: "Oak",
    description: "Warm paper and walnut",
    light: { bg: "#f6f4ef", panel: "#ece8df", raised: "#fffdf9", fg: "#1f1d1a", muted: "#6b665d", line: "#d9d3c7", accent: "#8a5a2b", accentFg: "#ffffff", danger: "#a23b2a" },
    dark: { bg: "#1b1a18", panel: "#23211e", raised: "#2a2825", fg: "#ece8e1", muted: "#a39d92", line: "#3a3732", accent: "#d29a5c", accentFg: "#1b1a18", danger: "#e0715f" },
  },
  {
    id: "slate",
    name: "Slate",
    description: "Cool grey with a clear blue",
    light: { bg: "#f4f6f8", panel: "#e8ecf0", raised: "#ffffff", fg: "#1a1f26", muted: "#5f6b78", line: "#d3dae2", accent: "#2f6fb3", accentFg: "#ffffff", danger: "#b3261e" },
    dark: { bg: "#15191e", panel: "#1c2128", raised: "#232a32", fg: "#e6ebf0", muted: "#93a0ad", line: "#313a44", accent: "#6aa8e8", accentFg: "#0d1520", danger: "#f2857b" },
  },
  {
    id: "sage",
    name: "Sage",
    description: "Soft greens, like a reading room",
    light: { bg: "#f3f5f1", panel: "#e6ebe3", raised: "#fcfdfb", fg: "#1b211c", muted: "#5f6b60", line: "#d2dace", accent: "#3d7a55", accentFg: "#ffffff", danger: "#a83a2c" },
    dark: { bg: "#161a17", panel: "#1d221e", raised: "#242a25", fg: "#e5ebe5", muted: "#98a598", line: "#333b34", accent: "#7cc398", accentFg: "#102016", danger: "#e8806f" },
  },
  {
    id: "ink",
    name: "Ink",
    description: "Black and white with a red accent",
    light: { bg: "#ffffff", panel: "#f2f2f2", raised: "#ffffff", fg: "#111111", muted: "#5c5c5c", line: "#dcdcdc", accent: "#c8102e", accentFg: "#ffffff", danger: "#c8102e" },
    dark: { bg: "#0e0e0e", panel: "#171717", raised: "#1f1f1f", fg: "#f2f2f2", muted: "#a3a3a3", line: "#2e2e2e", accent: "#ff5a6a", accentFg: "#160406", danger: "#ff7a7a" },
  },
];

const SYSTEM_UI = '"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif';

export const typeThemes: TypeTheme[] = [
  {
    id: "system",
    name: "System",
    description: "Windows' own Segoe UI",
    ui: SYSTEM_UI,
    display: '"Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif',
    cover: 'Georgia, "Times New Roman", serif',
  },
  {
    id: "clean",
    name: "Clean",
    description: "Inter throughout",
    ui: `"Inter Variable", ${SYSTEM_UI}`,
    display: `"Inter Variable", ${SYSTEM_UI}`,
    cover: `"Inter Variable", ${SYSTEM_UI}`,
  },
  {
    id: "bookish",
    name: "Bookish",
    description: "Literata titles, Source Sans text",
    ui: `"Source Sans 3 Variable", ${SYSTEM_UI}`,
    display: '"Literata Variable", Georgia, serif',
    cover: '"Literata Variable", Georgia, serif',
  },
  {
    id: "editorial",
    name: "Editorial",
    description: "Fraunces titles, IBM Plex Sans text",
    ui: `"IBM Plex Sans Variable", ${SYSTEM_UI}`,
    display: '"Fraunces Variable", Georgia, serif',
    cover: '"Fraunces Variable", Georgia, serif',
  },
];

export interface ThemeSettings {
  appearance: Appearance;
  colorTheme: string;
  typeTheme: string;
}

export const defaultThemeSettings: ThemeSettings = { appearance: "system", colorTheme: "oak", typeTheme: "system" };

/** The current choices and the resolved light or dark mode. Components read this. */
export const theme = $state({ ...defaultThemeSettings, mode: "light" as Mode });

const CACHE_KEY = "billy.theme";
const systemDark = typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)") : null;

export function colorTheme(id: string): ColorTheme {
  return colorThemes.find((t) => t.id === id) ?? colorThemes[0];
}

export function typeTheme(id: string): TypeTheme {
  return typeThemes.find((t) => t.id === id) ?? typeThemes[0];
}

/** The palette in use right now. */
export function currentPalette(): Palette {
  return colorTheme(theme.colorTheme)[theme.mode];
}

function resolveMode(appearance: Appearance): Mode {
  if (appearance === "system") return systemDark?.matches ? "dark" : "light";
  return appearance;
}

/** Applies theme settings to the page. */
export function applyTheme(settings: ThemeSettings) {
  Object.assign(theme, settings, { mode: resolveMode(settings.appearance) });
  const palette = currentPalette();
  const type = typeTheme(theme.typeTheme);
  const root = document.documentElement;
  const set = (name: string, value: string) => root.style.setProperty(name, value);
  set("--bg", palette.bg);
  set("--panel", palette.panel);
  set("--raised", palette.raised);
  set("--fg", palette.fg);
  set("--muted", palette.muted);
  set("--line", palette.line);
  set("--accent", palette.accent);
  set("--accent-fg", palette.accentFg);
  set("--danger", palette.danger);
  set(
    "--shadow",
    theme.mode === "dark"
      ? "0 1px 2px rgb(0 0 0 / 0.4), 0 2px 8px rgb(0 0 0 / 0.3)"
      : "0 1px 2px rgb(0 0 0 / 0.12), 0 2px 8px rgb(0 0 0 / 0.08)",
  );
  set("--font-ui", type.ui);
  set("--font-display", type.display);
  set("--font-cover", type.cover);
  root.style.colorScheme = theme.mode;
  root.dataset.mode = theme.mode;
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(settings));
  } catch {
    // Storage may be unavailable; the settings are also kept in Billy's database.
  }
}

/** Applies the last used theme straight away, before the database answers, to avoid a flash. */
export function applyCachedTheme() {
  let cached = defaultThemeSettings;
  try {
    cached = { ...defaultThemeSettings, ...JSON.parse(localStorage.getItem(CACHE_KEY) ?? "{}") };
  } catch {
    // Use the defaults.
  }
  applyTheme(cached);
  // Follow Windows when the appearance is "system".
  systemDark?.addEventListener("change", () => {
    if (theme.appearance === "system") applyTheme({ appearance: theme.appearance, colorTheme: theme.colorTheme, typeTheme: theme.typeTheme });
  });
}

export function readThemeSettings(raw: Record<string, unknown> | undefined): ThemeSettings {
  return { ...defaultThemeSettings, ...(raw as Partial<ThemeSettings>) };
}

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
  {
    id: "plum",
    name: "Plum",
    description: "Lavender greys and deep violet",
    light: { bg: "#f6f4f8", panel: "#ebe7f0", raised: "#fdfcfe", fg: "#211c27", muted: "#685f73", line: "#dbd4e3", accent: "#6b3fa0", accentFg: "#ffffff", danger: "#b0303a" },
    dark: { bg: "#19161d", panel: "#211d27", raised: "#29242f", fg: "#ebe6f1", muted: "#a59bb2", line: "#383141", accent: "#b48ae8", accentFg: "#1a1022", danger: "#ef8089" },
  },
  {
    id: "harbour",
    name: "Harbour",
    description: "Sea blues with a teal accent",
    light: { bg: "#f1f6f7", panel: "#e2ecee", raised: "#fbfdfd", fg: "#142326", muted: "#566b70", line: "#cddcdf", accent: "#0f7c82", accentFg: "#ffffff", danger: "#b3352b" },
    dark: { bg: "#11191b", panel: "#172124", raised: "#1e2a2d", fg: "#e2eef0", muted: "#8fa7ac", line: "#2a3a3e", accent: "#4fc3c9", accentFg: "#082022", danger: "#f0867c" },
  },
  {
    id: "dusk",
    name: "Dusk",
    description: "Wine-tinted greys with a rose accent",
    light: { bg: "#f8f4f6", panel: "#efe7ec", raised: "#fffdfe", fg: "#24171e", muted: "#6e5d67", line: "#e0d3db", accent: "#a8325c", accentFg: "#ffffff", danger: "#b3261e" },
    dark: { bg: "#1c1519", panel: "#251c22", raised: "#2d232a", fg: "#f1e6ec", muted: "#ad99a5", line: "#3e3139", accent: "#f2829f", accentFg: "#2a0d17", danger: "#ff8a80" },
  },
  {
    id: "ember",
    name: "Ember",
    description: "Charcoal with an orange glow",
    light: { bg: "#f7f6f5", panel: "#ecebe9", raised: "#ffffff", fg: "#1d1b1a", muted: "#66605b", line: "#dbd8d4", accent: "#c2410c", accentFg: "#ffffff", danger: "#b3261e" },
    dark: { bg: "#181716", panel: "#201f1d", raised: "#292725", fg: "#eeebe8", muted: "#a5a09a", line: "#36332f", accent: "#ff8f4d", accentFg: "#1f0d03", danger: "#ff7a6e" },
  },
  {
    id: "saffron",
    name: "Saffron",
    description: "Warm cream with a gold accent",
    light: { bg: "#faf7ee", panel: "#f1ebd9", raised: "#fffdf6", fg: "#221d10", muted: "#6d6550", line: "#e3dac2", accent: "#8a6100", accentFg: "#ffffff", danger: "#b3261e" },
    dark: { bg: "#1b1911", panel: "#232017", raised: "#2b281d", fg: "#f0ebdc", muted: "#aaa28b", line: "#3a3627", accent: "#f0c043", accentFg: "#231a00", danger: "#f2857b" },
  },
  {
    id: "olive",
    name: "Olive",
    description: "Khaki greens with an olive accent",
    light: { bg: "#f5f6ef", panel: "#e9ebdf", raised: "#fdfdf8", fg: "#1e2115", muted: "#636a54", line: "#d6dac8", accent: "#5f6f12", accentFg: "#ffffff", danger: "#a83a2c" },
    dark: { bg: "#181a13", panel: "#20231a", raised: "#282b21", fg: "#e9ecdf", muted: "#a0a78f", line: "#353a2b", accent: "#c3d461", accentFg: "#1a1f05", danger: "#e8806f" },
  },
  {
    id: "indigo",
    name: "Indigo",
    description: "Blue-violet greys with indigo",
    light: { bg: "#f5f5fb", panel: "#e9e9f5", raised: "#ffffff", fg: "#1b1b2e", muted: "#5f6078", line: "#d9d9ea", accent: "#4338ca", accentFg: "#ffffff", danger: "#b3261e" },
    dark: { bg: "#15151f", panel: "#1c1c29", raised: "#242433", fg: "#e8e8f5", muted: "#9c9cb8", line: "#303044", accent: "#a5a6ff", accentFg: "#15153a", danger: "#f2857b" },
  },
  {
    id: "midnight",
    name: "Midnight",
    description: "Deep navy with a gold accent",
    light: { bg: "#f3f5f9", panel: "#e4e9f1", raised: "#ffffff", fg: "#121a2b", muted: "#56627a", line: "#d0d8e5", accent: "#1f3f8f", accentFg: "#ffffff", danger: "#b3261e" },
    dark: { bg: "#0f1626", panel: "#152036", raised: "#1c2942", fg: "#e6ecf7", muted: "#93a1bd", line: "#2a3a58", accent: "#f2c14e", accentFg: "#1a1300", danger: "#ff8a80" },
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
  /** One of `typeThemes`, or "custom" for `customFont`. */
  typeTheme: string;
  /** A font installed on this computer, used everywhere when `typeTheme` is "custom". */
  customFont: string;
}

export const defaultThemeSettings: ThemeSettings = {
  appearance: "system",
  colorTheme: "oak",
  typeTheme: "system",
  customFont: "",
};

/** Keeps a typed font name safe to put in CSS: no quotes, backslashes or semicolons. */
export function cleanFontName(name: string): string {
  return name.replace(/["'\\;{}<>]/g, "").replace(/\s+/g, " ").trim().slice(0, 64);
}

/** A type theme that uses one font from this computer, falling back to Segoe UI. */
export function customTypeTheme(name: string): TypeTheme {
  const clean = cleanFontName(name);
  const stack = clean ? `"${clean}", ${SYSTEM_UI}` : SYSTEM_UI;
  return {
    id: "custom",
    name: "Custom",
    description: clean || "A font on this computer",
    ui: stack,
    display: stack,
    cover: stack,
  };
}

/** Whether a font is installed, judged by whether text in it measures differently from
 *  text in two very different fallback fonts. */
export function fontInstalled(name: string): boolean {
  const clean = cleanFontName(name);
  if (!clean) return false;
  const canvas = document.createElement("canvas").getContext("2d");
  if (!canvas) return true;
  const sample = "mmmmmmmmmmlli WQ 1234567890";
  return ["monospace", "serif"].some((fallback) => {
    canvas.font = `72px ${fallback}`;
    const base = canvas.measureText(sample).width;
    canvas.font = `72px "${clean}", ${fallback}`;
    return canvas.measureText(sample).width !== base;
  });
}

/** The current choices and the resolved light or dark mode. Components read this. */
export const theme = $state({ ...defaultThemeSettings, mode: "light" as Mode });

const CACHE_KEY = "billy.theme";
const systemDark = typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)") : null;

export function colorTheme(id: string): ColorTheme {
  return colorThemes.find((t) => t.id === id) ?? colorThemes[0];
}

export function typeTheme(id: string): TypeTheme {
  if (id === "custom") return customTypeTheme(theme.customFont);
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
    if (theme.appearance === "system") applyTheme(currentSettings());
  });
}

/** The current choices, without the resolved mode. */
export function currentSettings(): ThemeSettings {
  return {
    appearance: theme.appearance,
    colorTheme: theme.colorTheme,
    typeTheme: theme.typeTheme,
    customFont: theme.customFont,
  };
}

export function readThemeSettings(raw: Record<string, unknown> | undefined): ThemeSettings {
  return { ...defaultThemeSettings, ...(raw as Partial<ThemeSettings>) };
}

/**
 * Renderer-only appearance preference. Theme is pure presentation, so it lives
 * in the renderer (localStorage + a `data-theme` attribute) and never crosses
 * the native boundary or touches persisted `AppSettings`.
 */
export type ThemePreference = 'system' | 'light' | 'dark';

export const THEME_STORAGE_KEY = 'cachebite:theme';
export const DEFAULT_THEME: ThemePreference = 'system';

const THEME_VALUES: readonly ThemePreference[] = ['system', 'light', 'dark'];

export const isThemePreference = (value: unknown): value is ThemePreference =>
  typeof value === 'string' &&
  (THEME_VALUES as readonly string[]).includes(value);

type ReadableStorage = Pick<Storage, 'getItem'> | null | undefined;
type WritableStorage = Pick<Storage, 'setItem'> | null | undefined;
type ThemeRoot =
  | Pick<HTMLElement, 'setAttribute' | 'removeAttribute'>
  | null
  | undefined;

/** Read the stored preference, falling back to `system` on any failure. */
export function loadThemePreference(storage: ReadableStorage): ThemePreference {
  try {
    const raw = storage?.getItem(THEME_STORAGE_KEY);
    return isThemePreference(raw) ? raw : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME;
  }
}

/** Persist the preference; storage may be unavailable, so failures are swallowed. */
export function persistThemePreference(
  storage: WritableStorage,
  preference: ThemePreference,
): void {
  try {
    storage?.setItem(THEME_STORAGE_KEY, preference);
  } catch {
    // Storage disabled (private mode, quota). Preference simply falls back to
    // `system` on next load; not worth surfacing to the user.
  }
}

/**
 * Apply the preference to the document root. `system` removes the attribute so
 * the `prefers-color-scheme` media query drives the palette; an explicit value
 * pins it via a higher-specificity `:root[data-theme=...]` selector.
 */
export function applyThemePreference(
  root: ThemeRoot,
  preference: ThemePreference,
): void {
  if (!root) return;
  if (preference === 'system') root.removeAttribute('data-theme');
  else root.setAttribute('data-theme', preference);
}

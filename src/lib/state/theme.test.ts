import { describe, expect, it, vi } from 'vitest';
import {
  applyThemePreference,
  DEFAULT_THEME,
  isThemePreference,
  loadThemePreference,
  persistThemePreference,
  THEME_STORAGE_KEY,
} from './theme';

describe('theme preference', () => {
  it('recognizes only the three valid preferences', () => {
    expect(isThemePreference('system')).toBe(true);
    expect(isThemePreference('light')).toBe(true);
    expect(isThemePreference('dark')).toBe(true);
    expect(isThemePreference('neon')).toBe(false);
    expect(isThemePreference(null)).toBe(false);
    expect(isThemePreference(3)).toBe(false);
  });

  it('loads a stored preference', () => {
    const storage = { getItem: vi.fn().mockReturnValue('dark') };
    expect(loadThemePreference(storage)).toBe('dark');
    expect(storage.getItem).toHaveBeenCalledWith(THEME_STORAGE_KEY);
  });

  it('falls back to system for missing, invalid, or absent storage', () => {
    expect(loadThemePreference({ getItem: () => null })).toBe(DEFAULT_THEME);
    expect(loadThemePreference({ getItem: () => 'bogus' })).toBe(DEFAULT_THEME);
    expect(loadThemePreference(null)).toBe(DEFAULT_THEME);
  });

  it('falls back to system when reading storage throws', () => {
    expect(
      loadThemePreference({
        getItem: () => {
          throw new Error('blocked');
        },
      }),
    ).toBe(DEFAULT_THEME);
  });

  it('persists the preference and swallows storage failures', () => {
    const setItem = vi.fn();
    persistThemePreference({ setItem }, 'light');
    expect(setItem).toHaveBeenCalledWith(THEME_STORAGE_KEY, 'light');
    expect(() =>
      persistThemePreference(
        {
          setItem: () => {
            throw new Error('quota');
          },
        },
        'dark',
      ),
    ).not.toThrow();
    expect(() => persistThemePreference(null, 'dark')).not.toThrow();
  });

  it('applies explicit themes as an attribute and clears it for system', () => {
    const root = {
      setAttribute: vi.fn(),
      removeAttribute: vi.fn(),
    };
    applyThemePreference(root, 'dark');
    expect(root.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
    applyThemePreference(root, 'system');
    expect(root.removeAttribute).toHaveBeenCalledWith('data-theme');
    expect(() => applyThemePreference(null, 'dark')).not.toThrow();
  });
});

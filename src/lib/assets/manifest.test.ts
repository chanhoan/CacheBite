import { describe, expect, it } from 'vitest';

import { validatePetManifest } from './manifest';
import { requestedAnimationKey, resolvePetAnimation } from './resolver';

// The installer enumerates `resources/pets/` rather than a fixed list, so this
// does too: a pet added by dropping in a folder must not reach a release with
// an unvalidated manifest.
const bundledManifests = Object.entries(
  import.meta.glob('../../../src-tauri/resources/pets/*/manifest.json', {
    eager: true,
    import: 'default',
  }),
).map(
  ([path, manifest]) =>
    [path.replace(/^.*\/pets\/(.+)\/manifest\.json$/, '$1'), manifest] as [
      string,
      unknown,
    ],
);

const validManifest = {
  id: 'geometric-idle',
  displayName: 'Geometric Idle',
  defaultSize: { width: 128, height: 128 },
  animations: {
    idle: {
      type: 'frames',
      frames: ['frames/idle-01.svg', 'frames/idle-02.svg'],
      frameDurationMs: 160,
    },
  },
  states: { idle_exhausted: 'idle' },
};

describe('pet manifest validation', () => {
  it('finds the bundled packages on disk', () => {
    // Guards the glob itself: a zero-match would leave the suite below with no
    // cases and pass vacuously. Deliberately not an exact list — adding a pet
    // should extend the coverage, not break this test.
    expect(bundledManifests.length).toBeGreaterThan(0);
  });

  it.each(bundledManifests)(
    'accepts the generated %s package with all mood states',
    (id, candidate) => {
      const manifest = validatePetManifest(candidate);

      expect(manifest.id).toBe(id);
      expect(Object.keys(manifest.states)).toEqual([
        'idle',
        'idle_warn',
        'idle_critical',
        'idle_exhausted',
      ]);
    },
  );

  it('accepts a canonical manifest and returns an immutable value', () => {
    const manifest = validatePetManifest(validManifest);

    expect(manifest.id).toBe('geometric-idle');
    expect(manifest.animations.idle.type).toBe('frames');
    expect(Object.isFrozen(manifest)).toBe(true);
  });

  it.each([
    [{ ...validManifest, id: '../pet' }, 'id'],
    [{ ...validManifest, animations: {} }, 'idle'],
    [
      {
        ...validManifest,
        animations: {
          idle: {
            type: 'frames',
            frames: ['../secret.svg'],
            frameDurationMs: 160,
          },
        },
      },
      'path',
    ],
    [
      {
        ...validManifest,
        states: { excited: 'idle' },
      },
      'state',
    ],
    // Bare severity names are the superseded naming scheme: the resolver only
    // ever asks for the `idle_*` forms, so accepting these would let a package
    // declare frames that can never be shown.
    [{ ...validManifest, states: { warn: 'idle' } }, 'state'],
    [{ ...validManifest, states: { exhausted: 'idle' } }, 'state'],
  ])('rejects malformed manifests safely', (candidate, message) => {
    expect(() => validatePetManifest(candidate)).toThrow(message);
  });
});

describe('idle animation resolution', () => {
  it('resolves validated frames below the supplied package root', () => {
    const animation = resolvePetAnimation(
      validatePetManifest(validManifest),
      'asset://localhost/pets/geometric-idle/',
      'idle',
    );

    expect(animation).toEqual({
      type: 'frames',
      sources: [
        'asset://localhost/pets/geometric-idle/frames/idle-01.svg',
        'asset://localhost/pets/geometric-idle/frames/idle-02.svg',
      ],
      frameDurationMs: 160,
    });
  });

  it('resolves Windows Tauri asset URLs below the supplied package root', () => {
    const animation = resolvePetAnimation(
      validatePetManifest(validManifest),
      'http://asset.localhost/C%3A/Users/test/AppData/pets/cat/',
      'idle',
    );

    expect(animation).toMatchObject({
      sources: [
        'http://asset.localhost/C%3A/Users/test/AppData/pets/cat/frames/idle-01.svg',
        'http://asset.localhost/C%3A/Users/test/AppData/pets/cat/frames/idle-02.svg',
      ],
    });
  });

  it('rejects an invalid package root instead of resolving outside app data', () => {
    for (const root of [
      'javascript:alert(1)',
      'https://example.com/pet/',
      'http://localhost/pet/',
      'http://evil.example/pet/',
      'asset://evil.example/pet/',
    ]) {
      expect(() =>
        resolvePetAnimation(validatePetManifest(validManifest), root, 'idle'),
      ).toThrow('package root');
    }
  });
});

describe('v1.1 animation resolution', () => {
  it.each([
    [{ system: 'auth_required', mood: 'exhausted', dragging: true }, 'idle'],
    [{ system: 'error', mood: 'critical', dragging: true }, 'idle'],
    [{ system: 'loading', mood: 'warn', dragging: true }, 'idle'],
    [{ system: 'offline', mood: 'critical', dragging: false }, 'sleep'],
    [{ system: 'unavailable', mood: 'warn', dragging: true }, 'dragging'],
    [
      { system: 'active', mood: 'exhausted', dragging: false },
      'idle_exhausted',
    ],
    [{ system: 'active', mood: 'critical', dragging: false }, 'idle_critical'],
    [{ system: 'active', mood: 'warn', dragging: false }, 'idle_warn'],
    [{ system: 'active', mood: 'ok', dragging: false }, 'idle'],
  ] as const)('selects priority for %o', (context, expected) => {
    expect(requestedAnimationKey(context)).toBe(expected);
  });

  it('falls directly from a missing requested key to idle', () => {
    const manifest = validatePetManifest(validManifest);
    expect(
      resolvePetAnimation(
        manifest,
        'asset://localhost/pets/test/',
        'idle_critical',
      ),
    ).toEqual(
      resolvePetAnimation(manifest, 'asset://localhost/pets/test/', 'idle'),
    );
  });
});

import { describe, expect, it } from 'vitest';

import { validatePetManifest } from './manifest';
import { resolveIdleAnimation } from './resolver';
import { requestedAnimationKey, resolvePetAnimation } from './resolver';

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
  states: { exhausted: 'idle' },
};

describe('pet manifest validation', () => {
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
  ])('rejects malformed manifests safely', (candidate, message) => {
    expect(() => validatePetManifest(candidate)).toThrow(message);
  });
});

describe('idle animation resolution', () => {
  it('resolves validated frames below the supplied package root', () => {
    const animation = resolveIdleAnimation(
      validatePetManifest(validManifest),
      'asset://localhost/pets/geometric-idle/',
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

  it('rejects an invalid package root instead of resolving outside app data', () => {
    for (const root of [
      'javascript:alert(1)',
      'https://example.com/pet/',
      'http://localhost/pet/',
    ]) {
      expect(() =>
        resolveIdleAnimation(validatePetManifest(validManifest), root),
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
    ).toEqual(resolveIdleAnimation(manifest, 'asset://localhost/pets/test/'));
  });
});

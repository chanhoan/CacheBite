export const PET_STATES = [
  'ok',
  'warn',
  'critical',
  'exhausted',
  'idle_warn',
  'idle_critical',
  'idle_exhausted',
  'sleep',
  'dragging',
] as const;

export type PetState = (typeof PET_STATES)[number];

export interface FrameAnimationManifest {
  readonly type: 'frames';
  readonly frames: readonly string[];
  readonly frameDurationMs: number;
}

export interface ImageAnimationManifest {
  readonly type: 'image';
  readonly source: string;
}

export type AnimationManifest = FrameAnimationManifest | ImageAnimationManifest;

export interface PetManifest {
  readonly id: string;
  readonly displayName: string;
  readonly defaultSize: Readonly<{ width: number; height: number }>;
  readonly animations: Readonly<Record<string, AnimationManifest>> & {
    readonly idle: AnimationManifest;
  };
  readonly states: Readonly<Partial<Record<PetState, string>>>;
}

const PACKAGE_ID = /^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$/;
const ANIMATION_ID = /^[a-z][a-z0-9_]{0,63}$/;
const SAFE_ASSET_PATH =
  /^(?![/.])(?!.*(?:^|\/)\.\.?\/)(?!.*\\)[A-Za-z0-9_@+.,() -]+(?:\/[A-Za-z0-9_@+.,() -]+)*\.(?:gif|png|webp|svg)$/i;

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error(`Invalid pet manifest ${label}`);
  }
  return value as Record<string, unknown>;
}

function assetPath(value: unknown): string {
  if (typeof value !== 'string' || !SAFE_ASSET_PATH.test(value)) {
    throw new Error('Invalid pet manifest asset path');
  }
  return value;
}

function animation(value: unknown): AnimationManifest {
  const input = object(value, 'animation');
  if (input.type === 'image') {
    return Object.freeze({ type: 'image', source: assetPath(input.source) });
  }
  if (input.type !== 'frames')
    throw new Error('Invalid pet manifest animation type');
  if (!Array.isArray(input.frames) || input.frames.length === 0) {
    throw new Error('Invalid pet manifest animation frames');
  }
  if (
    typeof input.frameDurationMs !== 'number' ||
    !Number.isInteger(input.frameDurationMs) ||
    input.frameDurationMs < 16 ||
    input.frameDurationMs > 60_000
  ) {
    throw new Error('Invalid pet manifest frame timing');
  }
  return Object.freeze({
    type: 'frames',
    frames: Object.freeze(input.frames.map(assetPath)),
    frameDurationMs: input.frameDurationMs,
  });
}

export function validatePetManifest(value: unknown): PetManifest {
  const input = object(value, 'root');
  if (typeof input.id !== 'string' || !PACKAGE_ID.test(input.id)) {
    throw new Error('Invalid pet manifest id');
  }
  if (
    typeof input.displayName !== 'string' ||
    input.displayName.trim().length === 0 ||
    input.displayName.length > 80
  ) {
    throw new Error('Invalid pet manifest display name');
  }
  const size = object(input.defaultSize, 'default size');
  const dimensions = ['width', 'height'] as const;
  for (const dimension of dimensions) {
    if (
      typeof size[dimension] !== 'number' ||
      !Number.isInteger(size[dimension]) ||
      size[dimension] < 16 ||
      size[dimension] > 2048
    ) {
      throw new Error(`Invalid pet manifest default ${dimension}`);
    }
  }
  const animationInput = object(input.animations, 'animations');
  if (!Object.hasOwn(animationInput, 'idle')) {
    throw new Error('Invalid pet manifest: idle animation is required');
  }
  const animations: Record<string, AnimationManifest> = {};
  for (const [key, candidate] of Object.entries(animationInput)) {
    if (!ANIMATION_ID.test(key))
      throw new Error('Invalid pet manifest animation id');
    animations[key] = animation(candidate);
  }
  const stateInput =
    input.states === undefined ? {} : object(input.states, 'states');
  const states: Partial<Record<PetState, string>> = {};
  for (const [key, animationId] of Object.entries(stateInput)) {
    if (!PET_STATES.includes(key as PetState)) {
      throw new Error(`Invalid pet manifest state: ${key}`);
    }
    if (
      typeof animationId !== 'string' ||
      animations[animationId] === undefined
    ) {
      throw new Error(`Invalid pet manifest state animation: ${key}`);
    }
    states[key as PetState] = animationId;
  }
  return Object.freeze({
    id: input.id,
    displayName: input.displayName.trim(),
    defaultSize: Object.freeze({
      width: size.width as number,
      height: size.height as number,
    }),
    animations: Object.freeze(animations) as PetManifest['animations'],
    states: Object.freeze(states),
  });
}

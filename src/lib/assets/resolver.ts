import type { PetManifest } from './manifest';

export type ResolvedAnimation =
  | {
      readonly type: 'frames';
      readonly sources: readonly string[];
      readonly frameDurationMs: number;
    }
  | { readonly type: 'image'; readonly source: string };

export type RequestedAnimationKey =
  | 'idle'
  | 'idle_warn'
  | 'idle_critical'
  | 'idle_exhausted'
  | 'sleep'
  | 'dragging';
export interface AnimationContext {
  readonly system:
    | 'active'
    | 'auth_required'
    | 'unavailable'
    | 'error'
    | 'offline'
    | 'loading';
  readonly mood: 'ok' | 'warn' | 'critical' | 'exhausted';
  readonly dragging: boolean;
}

export function requestedAnimationKey(
  context: AnimationContext,
): RequestedAnimationKey {
  if (
    context.system === 'auth_required' ||
    context.system === 'error' ||
    context.system === 'loading'
  )
    return 'idle';
  if (context.dragging) return 'dragging';
  if (context.system === 'unavailable' || context.system === 'offline')
    return 'sleep';
  return context.mood === 'ok' ? 'idle' : `idle_${context.mood}`;
}

function validatedRoot(packageRoot: string): URL {
  let root: URL;
  try {
    root = new URL(packageRoot);
  } catch {
    throw new Error('Invalid pet package root');
  }
  const isAssetProtocol =
    root.protocol === 'asset:' &&
    root.hostname === 'localhost' &&
    root.port === '';
  const isWindowsAssetProtocol =
    root.protocol === 'http:' &&
    root.hostname === 'asset.localhost' &&
    root.port === '';
  if (!isAssetProtocol && !isWindowsAssetProtocol) {
    throw new Error('Invalid pet package root protocol');
  }
  if (root.username || root.password || root.search || root.hash) {
    throw new Error('Invalid pet package root');
  }
  let decodedPath: string;
  try {
    decodedPath = decodeURIComponent(root.pathname);
  } catch {
    throw new Error('Invalid pet package root path');
  }
  if (decodedPath.split('/').some((part) => part === '.' || part === '..')) {
    throw new Error('Invalid pet package root path');
  }
  if (!root.pathname.endsWith('/')) root.pathname += '/';
  return root;
}

export function resolvePetAnimation(
  manifest: PetManifest,
  packageRoot: string,
  requested: RequestedAnimationKey,
): ResolvedAnimation {
  const animationId =
    manifest.states[requested as keyof typeof manifest.states];
  const selected =
    animationId === undefined
      ? manifest.animations.idle
      : (manifest.animations[animationId] ?? manifest.animations.idle);
  const root = validatedRoot(packageRoot);
  if (selected.type === 'image')
    return Object.freeze({
      type: 'image',
      source: new URL(selected.source, root).href,
    });
  return Object.freeze({
    type: 'frames',
    sources: Object.freeze(
      selected.frames.map((frame) => new URL(frame, root).href),
    ),
    frameDurationMs: selected.frameDurationMs,
  });
}

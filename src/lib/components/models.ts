import type { ResolvedAnimation } from '../assets/resolver';
import type { Provider } from '../contracts/domain';
import type { Severity, SystemState } from '../state/engine';

export interface RingWindowModel {
  readonly usedPercent: number | null;
  readonly severity: Severity;
}

export type BadgeState = Exclude<SystemState, 'active'>;

/**
 * The overlay's small ring. Populated only when `ringMode` is `double`; its
 * provider is always the one the primary setting does *not* point at.
 */
export interface SatelliteRingModel {
  readonly provider: Provider;
  /** Display name used in the accessible label — 'Claude' | 'Codex'. */
  readonly providerName: string;
  readonly system: SystemState;
  readonly stale: boolean;
  readonly session: RingWindowModel;
  readonly weekly: RingWindowModel;
}

/** Which way round the outline the primary's mark walks. Chosen at random. */
export type OrbitDirection = 'forward' | 'reverse';

/**
 * The primary provider's mark, walking the outer edge. The pet says nothing
 * about which provider drives the overlay, so without this the satellite's
 * static logo would be the only branded thing on screen — which reads as if the
 * smaller ring were the more important one.
 */
export interface OrbitModel {
  readonly provider: Provider;
  /** Display name used in the accessible label — 'Claude' | 'Codex'. */
  readonly providerName: string;
  readonly direction: OrbitDirection;
}

export interface PetOverlayViewModel {
  readonly system: SystemState;
  readonly stale: boolean;
  readonly session: RingWindowModel;
  readonly weekly: RingWindowModel;
  readonly animation: ResolvedAnimation;
  readonly petName: string;
  /** Rendered edge length in CSS pixels, already clamped to the overlay window. */
  readonly size: number;
  /** `null` in single mode. The big ring stays bound to the primary either way. */
  readonly satellite: SatelliteRingModel | null;
  /** Present in both ring modes; the walked outline just gains the satellite. */
  readonly orbit: OrbitModel;
}

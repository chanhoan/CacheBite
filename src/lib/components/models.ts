import type { ResolvedAnimation } from '../assets/resolver';
import type { Severity, SystemState } from '../state/engine';

export interface RingWindowModel {
  readonly usedPercent: number | null;
  readonly severity: Severity;
}

export type BadgeState = Exclude<SystemState, 'active'>;

export interface PetOverlayViewModel {
  readonly system: SystemState;
  readonly stale: boolean;
  readonly session: RingWindowModel;
  readonly weekly: RingWindowModel;
  readonly animation: ResolvedAnimation;
  readonly petName: string;
}

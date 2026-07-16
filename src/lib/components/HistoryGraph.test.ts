import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import HistoryGraph from './HistoryGraph.svelte';

const samples = [
  {
    capturedAt: '2026-07-16T00:00:00Z',
    session: { usedPercent: 10, startsNewSegment: false },
    weekly: null,
  },
  {
    capturedAt: '2026-07-16T00:15:00Z',
    session: { usedPercent: 20, startsNewSegment: true },
    weekly: { usedPercent: 40, startsNewSegment: false },
  },
];

describe('HistoryGraph', () => {
  afterEach(cleanup);

  it('renders an accessible empty state and a labelled single point', async () => {
    const { rerender } = render(HistoryGraph, {
      props: { samples: [], window: 'session' },
    });
    expect(screen.getByText('No 5-hour history yet')).toBeTruthy();
    await rerender({ samples: [samples[0]!], window: 'session' });
    expect(
      screen.getByRole('img', { name: '5-hour usage history' }),
    ).toBeTruthy();
    expect(screen.getByLabelText('10% at 2026-07-16T00:00:00Z')).toBeTruthy();
  });

  it('breaks paths and uses native activation without double handling Enter', async () => {
    const onWindowChange = vi.fn();
    render(HistoryGraph, {
      props: { samples, window: 'session', onWindowChange },
    });
    expect(screen.getAllByTestId('history-segment')).toHaveLength(2);
    const weekly = screen.getByRole('tab', { name: 'Weekly' });
    weekly.focus();
    await fireEvent.keyDown(weekly, { key: 'Enter' });
    expect(onWindowChange).not.toHaveBeenCalled();
    await fireEvent.click(weekly);
    expect(onWindowChange).toHaveBeenCalledOnce();
    expect(onWindowChange).toHaveBeenCalledWith('weekly');
  });

  it('implements arrow navigation for the tablist', async () => {
    const onWindowChange = vi.fn();
    render(HistoryGraph, {
      props: { samples, window: 'session', onWindowChange },
    });
    const session = screen.getByRole('tab', { name: '5-hour' });
    const weekly = screen.getByRole('tab', { name: 'Weekly' });
    session.focus();
    await fireEvent.keyDown(session, { key: 'ArrowRight' });
    expect(onWindowChange).toHaveBeenCalledWith('weekly');
    expect(document.activeElement).toBe(weekly);
  });
});

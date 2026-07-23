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
    // The tablist now points at a real, focusable tabpanel named by its tab,
    // and individual marks are hidden so the series is announced once rather
    // than once per point.
    const panel = screen.getByRole('tabpanel', { name: '5-hour' });
    const tab = screen.getByRole('tab', { name: '5-hour' });
    expect(tab.getAttribute('aria-controls')).toBe(panel.id);
    expect(panel.getAttribute('aria-labelledby')).toBe(tab.id);
    expect(panel.getAttribute('tabindex')).toBe('0');
    expect(panel.querySelectorAll('circle')).toHaveLength(1);
    expect(screen.queryByLabelText('10% at 2026-07-16T00:00:00Z')).toBeNull();
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

  it('bounds rendered circles for a retained 3,000-sample history', () => {
    const longHistory = Array.from({ length: 3_000 }, (_, index) => ({
      capturedAt: `2026-07-16T00:${String(index).padStart(4, '0')}:00Z`,
      session: { usedPercent: index % 101, startsNewSegment: index === 1_500 },
      weekly: null,
    }));
    const { container } = render(HistoryGraph, {
      props: { samples: longHistory, window: 'session' },
    });

    expect(container.querySelectorAll('circle')).toHaveLength(240);
    expect(screen.getAllByTestId('history-segment')).toHaveLength(2);
    expect(
      screen.getByText('Showing a sampled view of 3,000 points.'),
    ).toBeTruthy();
  });
});

import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ProviderTabs from './ProviderTabs.svelte';

describe('ProviderTabs', () => {
  afterEach(cleanup);

  it('marks only the primary provider with an accessible star label', () => {
    render(ProviderTabs, { props: { selected: 'codex', primary: 'claude' } });

    expect(screen.getByRole('tab', { name: 'Claude (primary)' })).toBeTruthy();
    expect(screen.getByRole('tab', { name: 'Codex' })).toBeTruthy();
  });
});

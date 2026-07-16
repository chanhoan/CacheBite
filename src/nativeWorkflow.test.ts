import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const workflow = readFileSync('.github/workflows/native-smoke.yml', 'utf8');
const nativeSpec = readFileSync('tests/e2e/native.spec.ts', 'utf8');
const productionJob = workflow.split('  production-composition:')[1] ?? '';

describe('native production-composition workflow', () => {
  it('scrubs provider discovery inputs without replacing the command PATH', () => {
    expect(productionJob).toMatch(/CLAUDE_CODE_OAUTH_TOKEN:\s*['"]{2}/);
    expect(productionJob).toMatch(
      /CACHEBITE_CODEX_PATH:\s*\/tmp\/cachebite-no-codex/,
    );
    expect(productionJob).toMatch(/HOME:\s*\/tmp\/cachebite-production-home/);
    expect(productionJob).not.toMatch(/^\s+PATH:/m);
  });
});

describe('native production-composition spec', () => {
  it('returns to the overlay before opening the panel for provider assertions', () => {
    const productionCase =
      nativeSpec.split(
        "it('shows credential-free production provider states",
      )[1] ?? '';
    expect(
      productionCase.indexOf("switchWindow('CacheBite')"),
    ).toBeGreaterThanOrEqual(0);
    expect(productionCase.indexOf("switchWindow('CacheBite')")).toBeLessThan(
      productionCase.indexOf('main[data-window-label="overlay"]'),
    );
  });
});

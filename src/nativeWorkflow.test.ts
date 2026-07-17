import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const workflow = readFileSync('.github/workflows/native-smoke.yml', 'utf8');
const ciWorkflow = readFileSync('.github/workflows/ci.yml', 'utf8');
const releaseWorkflow = readFileSync('.github/workflows/release.yml', 'utf8');
const wdioConfig = readFileSync('wdio.conf.ts', 'utf8');
const nativeSpec = readFileSync('tests/e2e/native.spec.ts', 'utf8');
const refreshIpc = readFileSync('src-tauri/src/refresh/ipc.rs', 'utf8');
const windowModule = readFileSync('src-tauri/src/window/mod.rs', 'utf8');

function extractJob(source: string, name: string): string {
  const lines = source.split(/\r?\n/);
  const start = lines.findIndex((line) => line === `  ${name}:`);
  if (start < 0) return '';
  const endOffset = lines
    .slice(start + 1)
    .findIndex((line) => /^ {2}[a-zA-Z0-9_-]+:\s*$/.test(line));
  const end = endOffset < 0 ? undefined : start + 1 + endOffset;
  return lines.slice(start, end).join('\n');
}

function extractNamedStep(source: string, name: string): string {
  const lines = source.split(/\r?\n/);
  const start = lines.findIndex(
    (line) => line === `${' '.repeat(6)}- name: ${name}`,
  );
  if (start < 0) return '';
  const endOffset = lines
    .slice(start + 1)
    .findIndex((line) => /^ {6}- (?:name|run|uses):/.test(line));
  const end = endOffset < 0 ? undefined : start + 1 + endOffset;
  return lines.slice(start, end).join('\n');
}

function extractRunCommands(source: string): string[] {
  const lines = source.split(/\r?\n/);
  const commands: string[] = [];
  lines.forEach((line, index) => {
    const match = /^(\s*)(?:-\s+)?run:\s*(.*)$/.exec(line);
    if (!match) return;
    const indentation = match[1]?.length ?? 0;
    const command = match[2] ?? '';
    if (command !== '|' && command !== '>') {
      commands.push(command);
      return;
    }

    const endOffset = lines
      .slice(index + 1)
      .findIndex(
        (candidate) =>
          candidate.trim() !== '' &&
          candidate.length - candidate.trimStart().length <= indentation,
      );
    const end = endOffset < 0 ? undefined : index + 1 + endOffset;
    commands.push(
      lines
        .slice(index + 1, end)
        .map((candidate) => candidate.trim())
        .join(' '),
    );
  });
  return commands;
}

const productionJob = extractJob(workflow, 'production-composition');

describe('native production-composition workflow', () => {
  it('scrubs provider discovery inputs without replacing the command PATH', () => {
    expect(productionJob).toMatch(/CLAUDE_CODE_OAUTH_TOKEN:\s*['"]{2}/);
    expect(productionJob).toMatch(
      /CACHEBITE_CODEX_PATH:\s*\/tmp\/cachebite-no-codex/,
    );
    expect(productionJob).toMatch(
      /XDG_DATA_HOME:\s*\/tmp\/cachebite-production-home/,
    );
    expect(productionJob).toMatch(/HOME:\s*\/tmp\/cachebite-production-home/);
    expect(productionJob).not.toMatch(/^\s+PATH:/m);
  });

  it('runs credential-free production composition on Ubuntu and macOS', () => {
    expect(productionJob).toMatch(/runs-on:\s*\$\{\{\s*matrix\.os\s*\}\}/);
    const osLine = productionJob
      .split(/\r?\n/)
      .find((line) => /^\s+os:\s*\[/.test(line));
    expect(osLine).toBeDefined();
    const osValues = new Set(
      osLine!
        .slice(osLine!.indexOf('[') + 1, osLine!.lastIndexOf(']'))
        .split(',')
        .map((value) => value.trim()),
    );
    expect(osValues).toEqual(new Set(['ubuntu-latest', 'macos-latest']));
    expect(productionJob).toMatch(
      /Install native display prerequisites[\s\S]*if:\s*runner\.os == 'Linux'/,
    );
    expect(productionJob).toMatch(
      /Production-composition native smoke \(Linux\)[\s\S]*if:\s*runner\.os == 'Linux'[\s\S]*xvfb-run -a pnpm test:e2e/,
    );
    expect(productionJob).toMatch(
      /Production-composition native smoke \(macOS\)[\s\S]*if:\s*runner\.os != 'Linux'[\s\S]*pnpm test:e2e/,
    );
    const linuxStep = extractNamedStep(
      productionJob,
      'Production-composition native smoke (Linux)',
    );
    expect(linuxStep).toMatch(/^\s+HOME:\s*\/tmp\/cachebite-production-home/m);
  });
});

describe('native webdriver workflow', () => {
  it('uses the embedded webdriver provider on every native platform', () => {
    expect(wdioConfig).toMatch(/const driverProvider\s*=\s*['"]embedded['"];?/);
    expect(wdioConfig).not.toMatch(/driverProvider[\s\S]{0,120}official/);
    expect(wdioConfig).not.toContain('autoInstallTauriDriver');
  });

  it('builds every native smoke binary with webdriver enabled', () => {
    const buildCommands = workflow
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.includes('pnpm tauri build'));
    expect(buildCommands).toHaveLength(3);
    expect(
      buildCommands.every((line) =>
        /(?:^|\s)--features(?:=|\s+)webdriver(?:\s|$)/.test(line),
      ),
    ).toBe(true);
  });

  it('keeps webdriver disabled in production release builds', () => {
    const releaseBuildCommands = extractRunCommands(releaseWorkflow).filter(
      (command) => /\btauri build\b/.test(command),
    );
    expect(releaseBuildCommands).toHaveLength(2);
    expect(releaseBuildCommands).not.toEqual(
      expect.arrayContaining([
        expect.stringMatching(
          /--(?:all-features|features(?:=|\s+)webdriver)(?:\s|$)/,
        ),
      ]),
    );
  });
});

describe('dependency advisory workflow', () => {
  it('keeps Rust advisory scanning without the incompatible audit pin', () => {
    expect(ciWorkflow).toContain(
      'cargo install cargo-audit --version 0.22.2 --locked',
    );
    expect(ciWorkflow).toContain('cargo audit --file src-tauri/Cargo.lock');
    expect(ciWorkflow).not.toContain(
      'cargo install cargo-audit --version 0.21.2',
    );
  });
});

describe('native platform capability contract', () => {
  it('publishes a normalized operating-system value to the renderer', () => {
    expect(windowModule).toMatch(/pub os: &'static str/);
    expect(refreshIpc).toMatch(/platform_os\(std::env::consts::OS\)/);
    expect(windowModule).toMatch(/"macos" => "macos"/);
    expect(windowModule).toMatch(/"windows" => "windows"/);
    expect(windowModule).toMatch(/"linux" => "linux"/);
  });
});

describe('native production-composition spec', () => {
  it('retries labeled embedded-window discovery until its DOM is loaded', () => {
    expect(nativeSpec).toMatch(
      /browser\.waitUntil\(\s*async \(\) => \{[\s\S]*await browser\.getWindowHandles\(\)/,
    );
    expect(nativeSpec).toMatch(
      /handles\.find\(\(handle\) => handle === label\)/,
    );
  });

  it('returns to the overlay before opening the panel for provider assertions', () => {
    const productionCase =
      nativeSpec.split(
        "it('shows credential-free production provider states",
      )[1] ?? '';
    expect(
      productionCase.indexOf("switchToCacheBiteWindow('overlay')"),
    ).toBeGreaterThanOrEqual(0);
    expect(
      productionCase.indexOf("switchToCacheBiteWindow('overlay')"),
    ).toBeLessThan(productionCase.indexOf('main[data-window-label="overlay"]'));
  });

  it('allows documented non-Windows fullscreen capability degradation', () => {
    expect(nativeSpec).not.toContain(
      "expect.stringContaining('fullscreen detection is unavailable')",
    );
  });
});

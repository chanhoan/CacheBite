<script module>
  // SVG ids are document-global. A counter keeps two logos on one page (or two
  // renders in one test file) from sharing a gradient/mask definition.
  let nextInstance = 0;
</script>

<script>
  /** @type {{ provider: import('../contracts/domain').Provider }} */
  let { provider } = $props();

  const instance = nextInstance++;
  const gradientId = `cb-codex-gradient-${instance}`;
  const maskId = `cb-codex-mask-${instance}`;

  // Traced from docs/assets/provider-logos/claude-code.png (640x640, one flat
  // colour over transparency). The eyes are gaps between rects, not painted
  // shapes.
  const CLAUDE_RECTS = [
    { x: 80, y: 134, width: 480, height: 82 },
    { x: 80, y: 216, width: 80, height: 76 },
    { x: 200, y: 216, width: 240, height: 76 },
    { x: 480, y: 216, width: 80, height: 76 },
    { x: 0, y: 292, width: 640, height: 82 },
    { x: 80, y: 374, width: 480, height: 81 },
    { x: 120, y: 455, width: 40, height: 78 },
    { x: 200, y: 455, width: 40, height: 78 },
    { x: 400, y: 455, width: 40, height: 78 },
    { x: 480, y: 455, width: 40, height: 78 },
  ];

  // Six lobes at 60 degrees, offset 15 degrees, radius 68 at distance 30 from
  // centre. That reproduces the measured silhouette of
  // docs/assets/provider-logos/codex.png: 98 units at each lobe, 92 in the
  // valleys between them.
  const CODEX_LOBES = [15, 75, 135, 195, 255, 315].map((degrees) => {
    const radians = (degrees * Math.PI) / 180;
    return {
      cx: 100 + 30 * Math.cos(radians),
      cy: 100 + 30 * Math.sin(radians),
    };
  });
</script>

{#if provider === 'claude'}
  <svg
    class="logo"
    data-testid="provider-logo-claude"
    viewBox="0 0 640 640"
    aria-hidden="true"
  >
    {#each CLAUDE_RECTS as rect (`${rect.x}-${rect.y}`)}
      <rect
        x={rect.x}
        y={rect.y}
        width={rect.width}
        height={rect.height}
        fill="var(--logo-claude)"
      />
    {/each}
  </svg>
{:else}
  <svg
    class="logo"
    data-testid="provider-logo-codex"
    viewBox="0 0 200 200"
    aria-hidden="true"
  >
    <defs>
      <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="var(--logo-codex-from)" />
        <stop offset="1" stop-color="var(--logo-codex-to)" />
      </linearGradient>
      <!-- The prompt glyphs are knockouts in the source art, not painted
           strokes, so whatever sits behind the logo shows through them. -->
      <mask id={maskId}>
        <g fill="#fff">
          {#each CODEX_LOBES as lobe (lobe.cx)}
            <circle cx={lobe.cx} cy={lobe.cy} r="68" />
          {/each}
        </g>
        <g
          fill="none"
          stroke="#000"
          stroke-width="14"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M 55 73.4 L 70.6 100.8 L 55 128.2" />
          <path d="M 110.1 128.1 L 143 128.1" />
        </g>
      </mask>
    </defs>
    <rect
      width="200"
      height="200"
      fill={`url(#${gradientId})`}
      mask={`url(#${maskId})`}
    />
  </svg>
{/if}

<style>
  .logo {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>

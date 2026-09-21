// Shrink a nowrap heading's font-size until it fits its container on one
// line. Only ever shrinks below the CSS size; re-runs on resize and once
// webfonts have loaded. Usage: <h1 use:fit>…</h1>
export function fit(node: HTMLElement, opts: { min?: number } = {}) {
  const min = opts.min ?? 14;
  let raf = 0;
  const avail = () => {
    const p = node.parentElement;
    if (!p) return node.clientWidth;
    const cs = getComputedStyle(p);
    return p.clientWidth - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight);
  };
  const run = () => {
    node.style.fontSize = '';
    const css = parseFloat(getComputedStyle(node).fontSize);
    const w = avail();
    const need = node.scrollWidth;
    if (w > 0 && need > w) {
      let size = Math.max(min, Math.floor(css * (w / need) * 100) / 100);
      node.style.fontSize = `${size}px`;
      // Glyph widths don't scale perfectly linearly; nudge down until it truly fits.
      for (let i = 0; i < 4 && node.scrollWidth > w && size > min; i++) {
        size = Math.max(min, Math.floor(size * 0.985 * 100) / 100);
        node.style.fontSize = `${size}px`;
      }
    }
  };
  const schedule = () => { cancelAnimationFrame(raf); raf = requestAnimationFrame(run); };
  run();
  const ro = new ResizeObserver(schedule);
  ro.observe(node.parentElement ?? node);
  document.fonts?.ready.then(schedule);
  window.addEventListener('resize', schedule);
  return {
    destroy() { ro.disconnect(); window.removeEventListener('resize', schedule); cancelAnimationFrame(raf); }
  };
}

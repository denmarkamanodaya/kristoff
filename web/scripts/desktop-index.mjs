// The desktop build's index.html transform, used by scripts/build.mjs and tested on its own.

/**
 * The desktop index.html: no #gate at all (D-17, so it can never flash) and the launch screen
 * visible from the first paint. Fails the build if either marker moved.
 */
export function desktopIndex(html) {
  const gate = /<!-- Auth gate:[^\n]*-->\n<div id="gate"[\s\S]*?\n<\/div>\n/;
  const loading = '<div id="loading" class="loading" role="status" aria-live="polite" hidden>';
  if (!gate.test(html) || !html.includes(loading)) throw new Error('build: index.html gate or loading markers not found');
  return html.replace(gate, '').replace(loading, loading.replace(' hidden>', '>'));
}

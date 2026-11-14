// Waveform colours, read from Astryx theme tokens. A canvas can't resolve
// var(), so each token is applied to a hidden probe and read back resolved.

export interface WaveformPalette {
  unplayed: string;
  played: string;
  playhead: string;
  /** The minimap's visible-window shading, drawn at reduced alpha. */
  window: string;
  /** Beat Grid lines, drawn at reduced alpha; downbeats more strongly. */
  beat: string;
  downbeat: string;
}

const TOKENS: Record<keyof WaveformPalette, string> = {
  unplayed: "--color-border-emphasized",
  played: "--color-accent",
  playhead: "--color-text-primary",
  window: "--color-accent",
  beat: "--color-text-primary",
  downbeat: "--color-text-primary",
};

/** Resolves the palette's tokens as seen from `el`. */
export function readPalette(el: Element): WaveformPalette {
  const probe = document.createElement("span");
  probe.hidden = true;
  el.appendChild(probe);
  try {
    const read = (token: string) => {
      probe.style.color = `var(${token})`;
      return getComputedStyle(probe).color;
    };
    return {
      unplayed: read(TOKENS.unplayed),
      played: read(TOKENS.played),
      playhead: read(TOKENS.playhead),
      window: read(TOKENS.window),
      beat: read(TOKENS.beat),
      downbeat: read(TOKENS.downbeat),
    };
  } finally {
    probe.remove();
  }
}

// The loaded Track's Cues over the Waveform. View-only: clicking a marker
// seeks to it; Cues can't be created, edited or deleted here.
import type { CSSProperties } from "react";
import type { Cue } from "../../waveform/api";
import { cueColor, cueLabel, cueTitle, isHotCue } from "../../waveform/cues";
import { timeToMinimapX } from "../../waveform/geometry";

const colorStyle = (cue: Cue) =>
  ({ "--cue-color": cueColor(cue) }) as CSSProperties;

interface MarkersProps {
  cues: Cue[];
  onJump: (seconds: number) => void;
  isEnabled: boolean;
  /**
   * Each marker element by Cue id. The Waveform positions them in its own
   * draw, so markers and canvas move in the same frame, drag preview included.
   */
  elements: Map<string, HTMLElement>;
}

/** Cue markers over the zoomed view. Hidden until the Waveform places them. */
export function CueMarkers({
  cues,
  onJump,
  isEnabled,
  elements,
}: MarkersProps) {
  return (
    <div className="sb-cue-layer">
      {cues.map((cue) => {
        const label = cueLabel(cue);
        const title = cueTitle(cue);
        return (
          <button
            key={cue.id}
            type="button"
            ref={(el) => {
              if (el) elements.set(cue.id, el);
              else elements.delete(cue.id);
            }}
            className={`sb-cue ${isHotCue(cue) ? "is-hot" : "is-memory"}`}
            style={{ ...colorStyle(cue), display: "none" }}
            title={title}
            aria-label={title}
            disabled={!isEnabled}
            onPointerDown={(e) => e.stopPropagation()}
            onClick={() => onJump(cue.positionSec)}
          >
            <span className="sb-cue-line" />
            <span className="sb-cue-flag" aria-hidden="true">
              {label}
            </span>
          </button>
        );
      })}
    </div>
  );
}

/** Cue ticks over the minimap, which shows the whole Track. Not interactive. */
export function CueMinimapTicks({
  cues,
  duration,
}: {
  cues: Cue[];
  duration: number;
}) {
  if (duration <= 0) return null;
  return (
    <div className="sb-cue-layer">
      {cues.map((cue) =>
        cue.positionSec > duration ? null : (
          <span
            key={cue.id}
            className="sb-cue-tick"
            style={{
              ...colorStyle(cue),
              left: `${timeToMinimapX(cue.positionSec, 100, duration)}%`,
            }}
          />
        ),
      )}
    </div>
  );
}

// The Waveform zoom, remembered between sessions.
import { useCallback, useState } from "react";
import {
  DEFAULT_ZOOM,
  MAX_ZOOM,
  MIN_ZOOM,
  parseZoom,
  zoomIn as stepIn,
  zoomOut as stepOut,
} from "./zoom";

const ZOOM_KEY = "syncbox.zoom";

function readZoom(): number {
  try {
    return parseZoom(localStorage.getItem(ZOOM_KEY));
  } catch {
    return DEFAULT_ZOOM;
  }
}

function writeZoom(zoom: number) {
  try {
    localStorage.setItem(ZOOM_KEY, String(zoom));
  } catch {
    // Remembering the zoom is a convenience only.
  }
}

export function useZoom() {
  const [zoom, setZoomState] = useState(readZoom);

  const apply = useCallback((next: (z: number) => number) => {
    setZoomState((z) => {
      const v = next(z);
      writeZoom(v);
      return v;
    });
  }, []);

  const zoomIn = useCallback(() => apply(stepIn), [apply]);
  const zoomOut = useCallback(() => apply(stepOut), [apply]);
  const reset = useCallback(() => apply(() => DEFAULT_ZOOM), [apply]);

  return {
    zoom,
    zoomIn,
    zoomOut,
    reset,
    canZoomIn: zoom < MAX_ZOOM,
    canZoomOut: zoom > MIN_ZOOM,
  };
}

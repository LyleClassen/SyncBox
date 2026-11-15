// The loaded Track's Waveform: a minimap of the whole Track above a zoomed
// view that scrolls under a fixed centre playhead. Drawn on canvas from the
// stored Peaks, driven by Rust position events, coloured from theme tokens.
import { IconButton } from "@astryxdesign/core/IconButton";
import { Text } from "@astryxdesign/core/Text";
import {
  type PointerEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { PlaybackStatus } from "../../playback/api";
import type { Cue, WaveformData } from "../../waveform/api";
import { visibleCueX } from "../../waveform/cues";
import { drawMinimap, drawZoomed, MinimapCache } from "../../waveform/draw";
import {
  minimapXToTime,
  visibleWindow,
  zoomedXToTime,
} from "../../waveform/geometry";
import { readPalette, type WaveformPalette } from "../../waveform/palette";
import { peaksSource } from "../../waveform/source";
import type { WaveformStatus } from "../../waveform/status";
import { usePlayhead } from "../../waveform/usePlayhead";
import { useZoom } from "../../waveform/useZoom";
import { DEFAULT_ZOOM } from "../../waveform/zoom";
import { CueMarkers, CueMinimapTicks } from "./CueMarkers";

/** Pointer movement under this many px is a click, not a drag. */
const CLICK_SLOP = 4;

const NO_CUES: Cue[] = [];

const svg = {
  width: 16,
  height: 16,
  viewBox: "0 0 16 16",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round" as const,
};
const ZoomInIcon = () => (
  <svg {...svg} aria-hidden="true">
    <circle cx="7" cy="7" r="4.5" />
    <path d="M10.5 10.5l3 3M5 7h4M7 5v4" />
  </svg>
);
const ZoomOutIcon = () => (
  <svg {...svg} aria-hidden="true">
    <circle cx="7" cy="7" r="4.5" />
    <path d="M10.5 10.5l3 3M5 7h4" />
  </svg>
);
const ResetZoomIcon = () => (
  <svg {...svg} aria-hidden="true">
    <path d="M3 8a5 5 0 1 0 1.5-3.5" />
    <path d="M3 2.5V5h2.5" />
  </svg>
);

type View = "zoomed" | "minimap";

interface Drag {
  view: View;
  pointerId: number;
  /** clientX at pointerdown. */
  startClientX: number;
  /** px from the view's left edge at pointerdown. */
  startX: number;
  /** Playhead at pointerdown. */
  startPos: number;
  /** Where the playhead is drawn while dragging; sent on release. */
  preview: number;
  moved: boolean;
}

interface Props {
  data: WaveformData | null;
  peaksStatus: WaveformStatus;
  /** Latest position from Rust, in seconds. */
  position: number;
  status: PlaybackStatus;
  /** Seconds; 0 until a Track has loaded. */
  duration: number;
  onSeek: (seconds: number) => void;
  /** Why the requested Track couldn't be loaded. */
  error: { title: string; message: string } | null;
  /** No Track loaded or being loaded. */
  isEmpty: boolean;
}

export function Waveform({
  data,
  peaksStatus,
  position,
  status,
  duration,
  onSeek,
  error,
  isEmpty,
}: Props) {
  const rootRef = useRef<HTMLDivElement>(null);
  const minimapWrapRef = useRef<HTMLDivElement>(null);
  const zoomedWrapRef = useRef<HTMLDivElement>(null);
  const minimapRef = useRef<HTMLCanvasElement>(null);
  const zoomedRef = useRef<HTMLCanvasElement>(null);

  const zoom = useZoom();
  const getPlayhead = usePlayhead(position, status, duration);

  // Theme colours, resolved once the theme's CSS applies to this element.
  const [palette, setPalette] = useState<WaveformPalette | null>(null);
  useLayoutEffect(() => {
    if (rootRef.current) setPalette(readPalette(rootRef.current));
  }, []);

  const source = useMemo(
    () => (data && palette ? peaksSource(data, palette, duration) : null),
    [data, palette, duration],
  );
  const span = duration > 0 ? duration : (source?.duration ?? 0);
  const canSeek = !isEmpty && !error && span > 0 && status !== "loading";
  const isPlaying = status === "playing";
  const cues = data?.cues ?? NO_CUES;
  // Cues stay hidden under the error, empty and loading messages.
  const showCues =
    !isEmpty && !error && peaksStatus !== "failed" && peaksStatus !== "loading";

  // Drawing reads the latest inputs from here, so React renders never draw.
  const live = useRef({
    source,
    palette,
    zoom: zoom.zoom,
    span,
    isPlaying,
    cues,
  });
  live.current = { source, palette, zoom: zoom.zoom, span, isPlaying, cues };
  const cueEls = useRef(new Map<string, HTMLElement>());
  const drag = useRef<Drag | null>(null);
  const dpr = useRef(1);
  const cache = useMemo(() => new MinimapCache(), []);
  const frame = useRef(0);

  const drawNow = useCallback(() => {
    const l = live.current;
    const zc = zoomedRef.current;
    const mc = minimapRef.current;
    if (!l.palette || !zc || !mc) return;
    const at = drag.current?.preview ?? getPlayhead(performance.now());
    const zctx = zc.getContext("2d");
    if (zctx) {
      drawZoomed(zctx, {
        source: l.source,
        width: zc.width,
        height: zc.height,
        dpr: dpr.current,
        position: at,
        zoom: l.zoom,
        palette: l.palette,
      });
    }
    const mctx = mc.getContext("2d");
    if (mctx) {
      drawMinimap(mctx, {
        source: l.source,
        duration: l.span,
        width: mc.width,
        height: mc.height,
        dpr: dpr.current,
        position: at,
        window: visibleWindow(at, zc.width / dpr.current, l.zoom),
        palette: l.palette,
        cache,
      });
    }
    // Cue markers move with the canvas, in CSS px.
    const cssWidth = zc.width / dpr.current;
    for (const cue of l.cues) {
      const el = cueEls.current.get(cue.id);
      if (!el) continue;
      const x = visibleCueX(cue, cssWidth, at, l.zoom);
      if (x === null) {
        el.style.display = "none";
      } else {
        el.style.display = "";
        el.style.transform = `translateX(${x}px)`;
      }
    }
  }, [getPlayhead, cache]);

  // One frame on request; keeps going while playing.
  const requestDraw = useCallback(() => {
    if (frame.current) return;
    const tick = () => {
      frame.current = 0;
      drawNow();
      if (live.current.isPlaying) frame.current = requestAnimationFrame(tick);
    };
    frame.current = requestAnimationFrame(tick);
  }, [drawNow]);

  useEffect(
    () => () => {
      cancelAnimationFrame(frame.current);
      frame.current = 0;
    },
    [],
  );

  // Redraw when anything drawn changes.
  // biome-ignore lint/correctness/useExhaustiveDependencies: these are the inputs drawNow reads through refs.
  useEffect(() => {
    requestDraw();
  }, [
    requestDraw,
    source,
    palette,
    zoom.zoom,
    span,
    position,
    status,
    cues,
    showCues,
  ]);

  // Canvas backing stores follow their element size and the device pixel
  // ratio, which changes when the window moves to another monitor.
  useEffect(() => {
    const pairs = [
      [minimapWrapRef.current, minimapRef.current],
      [zoomedWrapRef.current, zoomedRef.current],
    ] as const;
    const size = () => {
      dpr.current = window.devicePixelRatio || 1;
      for (const [wrap, canvas] of pairs) {
        if (!wrap || !canvas) continue;
        const w = Math.max(1, Math.round(wrap.clientWidth * dpr.current));
        const h = Math.max(1, Math.round(wrap.clientHeight * dpr.current));
        if (canvas.width !== w) canvas.width = w;
        if (canvas.height !== h) canvas.height = h;
      }
      // Resizing clears a canvas, so draw now rather than next frame.
      drawNow();
    };

    const observer = new ResizeObserver(size);
    for (const [wrap] of pairs) if (wrap) observer.observe(wrap);

    let query: MediaQueryList | null = null;
    const onDprChange = () => {
      size();
      watchDpr();
    };
    const watchDpr = () => {
      query?.removeEventListener("change", onDprChange);
      query = window.matchMedia(
        `(resolution: ${window.devicePixelRatio || 1}dppx)`,
      );
      query.addEventListener("change", onDprChange);
    };
    watchDpr();
    size();

    return () => {
      observer.disconnect();
      query?.removeEventListener("change", onDprChange);
    };
  }, [drawNow]);

  // Seeking: a click seeks to the time under the pointer. Dragging the zoomed
  // view pans the Waveform under the fixed playhead; dragging the minimap
  // follows the pointer. Either way the seek is sent once, on release.
  const pointerDown = (view: View) => (e: PointerEvent<HTMLCanvasElement>) => {
    if (!canSeek || e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const x = e.clientX - e.currentTarget.getBoundingClientRect().left;
    const startPos = getPlayhead(performance.now());
    drag.current = {
      view,
      pointerId: e.pointerId,
      startClientX: e.clientX,
      startX: x,
      startPos,
      preview:
        view === "minimap"
          ? minimapXToTime(x, e.currentTarget.clientWidth, span)
          : startPos,
      moved: false,
    };
    requestDraw();
  };

  const pointerMove = (e: PointerEvent<HTMLCanvasElement>) => {
    const d = drag.current;
    if (!d || d.pointerId !== e.pointerId) return;
    const dx = e.clientX - d.startClientX;
    if (Math.abs(dx) >= CLICK_SLOP) d.moved = true;
    if (!d.moved) return;
    if (d.view === "zoomed") {
      d.preview = Math.min(span, Math.max(0, d.startPos - dx / zoom.zoom));
    } else {
      const x = e.clientX - e.currentTarget.getBoundingClientRect().left;
      d.preview = minimapXToTime(x, e.currentTarget.clientWidth, span);
    }
    requestDraw();
  };

  const pointerUp = (e: PointerEvent<HTMLCanvasElement>) => {
    const d = drag.current;
    if (!d || d.pointerId !== e.pointerId) return;
    drag.current = null;
    const target =
      d.view === "zoomed" && !d.moved
        ? zoomedXToTime(
            d.startX,
            e.currentTarget.clientWidth,
            d.startPos,
            zoom.zoom,
            span,
          )
        : d.preview;
    onSeek(target);
    requestDraw();
  };

  const pointerCancel = (e: PointerEvent<HTMLCanvasElement>) => {
    if (drag.current?.pointerId !== e.pointerId) return;
    drag.current = null;
    requestDraw();
  };

  const handlers = (view: View) => ({
    onPointerDown: pointerDown(view),
    onPointerMove: pointerMove,
    onPointerUp: pointerUp,
    onPointerCancel: pointerCancel,
    onLostPointerCapture: pointerCancel,
  });

  let overlay: ReactNode = null;
  if (error) {
    overlay = (
      <div className="sb-waveform-overlay sb-waveform-error" role="alert">
        <Text type="label" color="inherit">
          {error.title}
        </Text>
        <Text type="supporting" color="inherit">
          {error.message}
        </Text>
      </div>
    );
  } else if (isEmpty) {
    overlay = (
      <div className="sb-waveform-overlay">
        <Text type="supporting">Double-click a Track to load it</Text>
      </div>
    );
  } else if (peaksStatus === "failed") {
    overlay = (
      <div className="sb-waveform-overlay">
        <Text type="supporting">Couldn't draw the Waveform for this Track</Text>
      </div>
    );
  } else if (peaksStatus === "loading") {
    overlay = (
      <div className="sb-waveform-overlay">
        <Text type="supporting">Drawing the Waveform…</Text>
      </div>
    );
  }

  const seekClass = canSeek ? "" : " is-disabled";

  return (
    <div className="sb-waveform" ref={rootRef}>
      <div className={`sb-waveform-minimap${seekClass}`} ref={minimapWrapRef}>
        <canvas
          ref={minimapRef}
          role="img"
          aria-label="Waveform overview"
          {...handlers("minimap")}
        />
        {showCues && <CueMinimapTicks cues={cues} duration={span} />}
      </div>
      <div className={`sb-waveform-zoomed${seekClass}`} ref={zoomedWrapRef}>
        <canvas
          ref={zoomedRef}
          role="img"
          aria-label="Waveform"
          {...handlers("zoomed")}
        />
        {showCues && (
          <CueMarkers
            cues={cues}
            elements={cueEls.current}
            isEnabled={canSeek}
            onJump={(s) => onSeek(Math.min(span, Math.max(0, s)))}
          />
        )}
        <div className="sb-waveform-zoom">
          <IconButton
            label="Zoom in"
            tooltip="Zoom in"
            icon={<ZoomInIcon />}
            variant="secondary"
            size="sm"
            isDisabled={!zoom.canZoomIn}
            onClick={zoom.zoomIn}
          />
          <IconButton
            label="Zoom out"
            tooltip="Zoom out"
            icon={<ZoomOutIcon />}
            variant="secondary"
            size="sm"
            isDisabled={!zoom.canZoomOut}
            onClick={zoom.zoomOut}
          />
          <IconButton
            label="Reset zoom"
            tooltip={`Reset zoom (${DEFAULT_ZOOM} px/s)`}
            icon={<ResetZoomIcon />}
            variant="secondary"
            size="sm"
            isDisabled={zoom.zoom === DEFAULT_ZOOM}
            onClick={zoom.reset}
          />
        </div>
        {overlay}
      </div>
    </div>
  );
}

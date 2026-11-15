import { IconButton } from "@astryxdesign/core/IconButton";
import { Slider } from "@astryxdesign/core/Slider";
import { Text } from "@astryxdesign/core/Text";
import { useEffect } from "react";
import { formatTime } from "../../playback/time";
import type { Playback } from "../../playback/usePlayback";
import { useWaveformData } from "../../waveform/useWaveformData";
import { Waveform } from "./Waveform";

// Astryx has no transport icons, so these are drawn here at its icon size.
const svg = {
  width: 16,
  height: 16,
  viewBox: "0 0 16 16",
};
const PlayIcon = () => (
  <svg {...svg} aria-hidden="true" fill="currentColor">
    <path d="M4 2.5v11l9-5.5z" />
  </svg>
);
const PauseIcon = () => (
  <svg {...svg} aria-hidden="true" fill="currentColor">
    <rect x="3" y="2.5" width="3.5" height="11" rx="0.5" />
    <rect x="9.5" y="2.5" width="3.5" height="11" rx="0.5" />
  </svg>
);
const VolumeIcon = () => (
  <svg
    {...svg}
    aria-hidden="true"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
  >
    <path d="M2.5 6h2.5l3.5-3v10l-3.5-3H2.5z" fill="currentColor" />
    <path d="M11 5.5a3.5 3.5 0 0 1 0 5M12.8 3.5a6 6 0 0 1 0 9" />
  </svg>
);

/** Space toggles play/pause, except while typing in a field. */
function useSpaceToggle(toggle: () => void, isEnabled: boolean) {
  useEffect(() => {
    if (!isEnabled) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.code !== "Space" || e.repeat) return;
      const el = e.target as HTMLElement | null;
      if (
        el &&
        (el.isContentEditable ||
          ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName))
      ) {
        return;
      }
      // Also stops Space activating a focused button, which would toggle twice.
      e.preventDefault();
      toggle();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [toggle, isEnabled]);
}

interface Props {
  playback: Playback;
  /** Title and artist of the requested Track, if it's the loaded one. */
  title: string | null;
  artist: string | null;
  /** Why the requested Track couldn't be loaded, shown over the Waveform. */
  loadError: { title: string; message: string } | null;
}

export function PlayerBar({ playback, title, artist, loadError }: Props) {
  const { state, position, volume } = playback;
  const isLoaded = state.trackId !== null && state.status !== "loading";
  const isPlaying = state.status === "playing";
  useSpaceToggle(playback.toggle, isLoaded);
  // Peaks and Cues for the loaded Track, kept current as Peaks are computed.
  const waveform = useWaveformData(isLoaded ? state.trackId : null);

  const heading =
    state.status === "loading"
      ? "Loading…"
      : state.trackId
        ? title || "Untitled"
        : "No Track loaded";

  return (
    <section className="sb-player" aria-label="Player">
      <Waveform
        data={waveform.data}
        peaksStatus={waveform.status}
        position={position}
        status={state.status}
        duration={state.duration}
        onSeek={playback.seek}
        error={loadError}
        isEmpty={state.trackId === null && state.status !== "loading"}
      />
      <div className="sb-player-controls">
        <IconButton
          label={isPlaying ? "Pause" : "Play"}
          tooltip={isPlaying ? "Pause (Space)" : "Play (Space)"}
          icon={isPlaying ? <PauseIcon /> : <PlayIcon />}
          variant="primary"
          isDisabled={!isLoaded}
          onClick={playback.toggle}
        />
        <div className="sb-player-track">
          <Text type="label" maxLines={1}>
            {heading}
          </Text>
          <Text type="supporting" maxLines={1}>
            {state.trackId && state.status !== "loading"
              ? artist || " "
              : "Double-click a Track to load it"}
          </Text>
        </div>
        <div className="sb-player-time">
          <Text type="supporting" hasTabularNumbers>
            {isLoaded ? formatTime(Math.min(position, state.duration)) : "–"}
          </Text>
          <Text type="supporting">/</Text>
          <Text type="supporting" hasTabularNumbers>
            {isLoaded ? formatTime(state.duration) : "–"}
          </Text>
        </div>
        <div className="sb-player-volume">
          <VolumeIcon />
          <Slider
            label="Volume"
            isLabelHidden
            min={0}
            max={1}
            step={0.01}
            value={volume}
            formatValue={(v) => `${Math.round(v * 100)}%`}
            width={120}
            onChange={playback.setVolume}
          />
        </div>
      </div>
    </section>
  );
}

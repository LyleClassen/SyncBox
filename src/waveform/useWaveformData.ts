// The data behind the loaded Track's Waveform. Peaks are computed in Rust on
// a Track's first load; until then `data.peaks` is null and the status is
// "loading". A "ready" event triggers a refetch.
import { useEffect, useState } from "react";
import * as api from "./api";
import { isForTrack, statusOf, type WaveformStatus } from "./status";

interface State {
  trackId: string | null;
  data: api.WaveformData | null;
  status: WaveformStatus;
}

const IDLE: State = { trackId: null, data: null, status: "idle" };

export function useWaveformData(trackId: string | null) {
  const [state, setState] = useState<State>(IDLE);

  useEffect(() => {
    if (trackId === null) {
      setState(IDLE);
      return;
    }
    let disposed = false;
    setState({ trackId, data: null, status: "loading" });

    const fetch = () =>
      api
        .getWaveformData(trackId)
        .then((data) => {
          if (disposed) return;
          setState((prev) => ({
            trackId,
            data,
            status: statusOf(
              data,
              prev.trackId === trackId ? prev.status : "loading",
            ),
          }));
        })
        .catch(() => {
          if (!disposed) setState({ trackId, data: null, status: "failed" });
        });

    // Subscribe before fetching, so a "ready" in between isn't missed.
    const subscriptions = [
      api.onPeaksReady((e) => {
        if (!disposed && isForTrack(e.trackId, trackId)) fetch();
      }),
      api.onPeaksFailed((e) => {
        if (!disposed && isForTrack(e.trackId, trackId)) {
          setState((prev) =>
            prev.trackId === trackId && prev.status !== "ready"
              ? { ...prev, status: "failed" }
              : prev,
          );
        }
      }),
    ];
    Promise.all(subscriptions).then(() => {
      if (!disposed) fetch();
    });

    return () => {
      disposed = true;
      for (const s of subscriptions) s.then((unlisten) => unlisten());
    };
  }, [trackId]);

  // Never hand back another Track's data while the new one is fetched.
  const current = state.trackId === trackId ? state : IDLE;
  return { data: current.data, status: current.status };
}

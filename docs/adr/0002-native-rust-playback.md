# Play audio in Rust, not in the webview

mixxxa played Tracks through an `<audio>` element (via wavesurfer.js), streamed from a local HTTP server. SyncBox decodes and plays audio in the Rust process instead (`symphonia` to decode, `cpal`/`rodio` to output). The webview only shows the position and sends commands. The deciding reason is format support: DJ collections are often AIFF, and Chromium/WebView2 can't decode AIFF, so webview playback would leave those Tracks silent. Rust playback also makes later features possible, such as choosing the output device and sending a pre-listen to a separate output, which a browser audio element can't do.

## Consequences

The webview can no longer decode audio, so waveform peaks must also be computed in Rust. wavesurfer.js, if kept at all, becomes a renderer only.

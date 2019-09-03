As a byproduct of experimenting with CPAL, the most popular audio abstraction
layer currently available in Rust, I wrote a little program that makes a sound
reminiscent of an organ.

I'll likely write more interesting code later on, but not necessarily with CPAL,
because it has three major issues from my perspective:

- It only supports old ALSA on Linux. No PulseAudio and, worse, no JACK.
- It provides zero timing information on the audio streams (no timestamp, no
  latency estimate), which hampers things like AV sync and latency compensation.
- It provides zero guarantee of synchronization between audio streams (unlike,
  say, PortAudio or JACK, which call you for all streams of a given device at
  once, leaving ugly stream sync for exotic multi-device use cases only).

Fixing this myself would require making myself familiar with the relevant APIs
from Apple and Microsoft, which I have no interest in as long as I'm just
playing around. For now, I'll just use JACK/Pulse/PortAudio bindings directly.

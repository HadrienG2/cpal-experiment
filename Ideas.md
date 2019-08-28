Even the classic substractive synth seems to allow feedback.

            _______
    VCO -> |       |     _____      _____
           |       |    |     |    |     |
    VCO -> | MIXER | -> | VCF | -> | VCA | --> OUTPUT
           |       |    |_____|    |_____|  |
       |-> |_______|                        |
       |____________________________________|

Feedback seems tricky to handle:
* Output requests a buffer
* Engine requests a frame from VCA
* VCA requests a frame from VCF
* VCF requests a frame from mixer
* Mixer requests a frame from VCO1, VCO2... and output
* What next?

---

From PulseAudio's documentation, it seems that some audio backends gives us no
choice w.r.t. output format, including buffer size and sample format. Currently,
cpal does not allow us to choose the buffer size anyhow.

This makes it difficult to plan our allocations in advance. Further, in backends
that allow output redirection (e.g. jack, possibly pulseaudio as well), output
format may change without warning.

It would be desirable to enforce a single internal format (e.g. f32) that we
optimize for, and then convert to whatever the hardware expects at the end. The
sample library could be useful for this. But I probably don't want to use it all
the way through, because some of its design decisions seem simd-hostile
(abstraction over sample type, iterator-based design...).

Further, this also allows us to abstract multichannel output into N mono channel
which is more convenient in a modular context.

Can sampling rate change as well? Probably. That's a pain. I may want to enforce
a certain internal sampling rate which is decoupled from the hardware-mandated
one, and provide asynchronous transitions of components from one sampling rate
to another. Sounds tricky.

Alternatively, I could operate in terms of time (e.g. nanoseconds since epoch),
though the non-integer relationship between samples and times may come to bite
me later on. It would also cost a bit of performance, but probably nothing worth
worrying about.

I definitely don't want to handle time as a float, adding small quantities to
an indefinitely growing float is a recipe for disaster.

---

In a modular context, we'll need to handle modulation signals in addition to
audio signals. &[f32] is probably enough for both? But need to define some kind
of standard, e.g. 0.0 -> 1.0 for unipolar and -1.0 -> 1.0 for bipolar.

For pitch, 1V/octave is common, and I'll probably want to steal that into
1.0 per octave in a floating-point context.

Is f32 enough? That's \~24 bits of precision, pro audio fixed-point hardware
doesn't do any better (and generally does worse, because your low-order bits are
in the noise floor), so I'll take 2x SIMD processing power and memory bandwidth
over f64, thanks.

---

Thread priority is a concern... which cpal doesn't give me control over, so it's
maybe not worth worrying about now. When we do need to care about it, the
audio-thread-priority crate seems like something worth investigating.

Memory locking is something we also want, and that one seems easy to handle
using the region crate.

---

Can I have multiple outputs? If so, I'll need to handle multiple clocks. That
can be tricky for modules that keep internal state like delays. In general,
changing sampling rate will also be tricky for these. I may want to enforce
a certain high internal sampling rate that can never change. Or maybe that can
change rarely? Not the same thing at all!

---

Random thoughts on the signal path:

- I have a bunch of modules, which have inputs and outputs.
- Each input may be connected to exactly one output. Each output can be
  connected to multiple inputs.
- Inputs and outputs exchange data via buffers of f32 at a given sampling rate.
- Since there is a common sampling rate, there is also a common clock shared by
  all buffers: no synchronization!
- Feedback should be initially forbidden, but I may want to support it later on.
- Initially, I'll support only monophonic buffers, later I may want to support
  multichannel. Should plan ahead for it.
- Initially, I'll support only one audio output, but later I may want to support
  multiple outputs. This raises interesting problems. For example, the clock of
  one audio interface may have jitter w.r.t. another, which means that whatever
  handles the interface to the outside world may need to buffer audio frames
  from the past until all output interfaces have fetched it. The garbage
  collection for this could get tricky. And then there's of course the fact that
  different interfaces may not agree on a single sampling rate and pick
  annoyingly close values like 44.1kHz vs 48kHz, which is bound to cause
  resampling artifacts unless the internal sampling rate is much higher.
- Audio output should really, really be decoupled from the internal processing
  given all the mess that it creates.
- How should I do multi-threading? If audio processing is a DAG, task-based
  parallelism a la rayon/TBB sounds like a nice way to do it, where every task
  is a node and dependencies are handled somehow. But doing it w/o allocations
  is hard.
- There should be some way to request "eventual" sample rate changes, knowing
  that for things like delays it's gonna require internal memory conversions. Or
  we could do it the JACK way and say that you must restart the system before
  you can change the sample rate.
- I'll eventually want to support audio input. How do I handle the fact that in
  cpal at least, it's decoupled from output, given that I need input to generate
  output?
- How do I do polyphony? Isn't that basically duplicating the signal path, but
  sharing the rest like e.g. module configuration? Could multichannel help here?
- As soon as I have more than one input/output, if I operate in the CPAL model,
  I must decouple my internal clock from the device clocks. This is gonna make
  my life hard as I need to handle asynchronous RT scheduling, but it also has
  serious advantages. For example, I can provide samples more quickly when the
  hardware requests them (through lookahead), I can use smaller internal buffer
  sizes than what the hardware expects (yielding smaller input-to-audio
  latencies)...

---

General idea:

    Input1 -> | Source | -> |      | -> | Sink | -> Output1
                            | Work |
    Input2 -> | Source | -> |      | -> | Sink | -> Output2

* Sources are magic from the engine's point of view. Internally, they answer
  input device callback and take care of resampling and buffering.
* Sinks are also magic from the engine's point of view. Internally, they
  resample and buffer, and they answer output device callback.
* All engine-visible communication occurs via fixed-size data packets, called
  periods, buffers, or blocks, whichever you like best. Block size is uniform
  throughout the pipeline. If you want more buffering, you need to take care of
  it internally.
    - Alternative: Use bounded queues explicitly with high and low water marks?
* Each module keeps track of an internal sample-based clock, which advances as
  output is produced. Output will only be produced once, if you need to see it
  multiple times (e.g. for multiple output devices) you must buffer it yourself.
  Without this property, things like delays and filters would be near impossible
  to implement.
* Must figure out a way to handle diverging clocks and buffer sizes across
  devices. Can't just assume that all devices share a common clock? But that
  could lead to some samples of delay across inputs and outputs... JACK has what
  we need here, how about others? May provide API to support desynchronization,
  but assume things to be sync?
* Basic algorithm that might actually work: when receiving a buffer of N frames
  at sample rate R from device D, assume last frame is at current timestamp,
  i.e. delay between time where hardware recorded last frame and audio thread is
  woken up is zero. In the real world, this may work out because processing
  delays are quick and identical for every input.
* Lookahead time should be able to change dynamically, because we'll need more
  lookahead if a HW interface with a larger buffer size is connected as an
  output.
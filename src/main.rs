use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};
use std::f32::consts::PI;

fn main() {
    let host = cpal::default_host();
    println!("Selected host: {:?}", host.id());

    let event_loop = host.event_loop();

    let device = host.default_output_device()
                     .expect("Failed to find an audio output");
    println!("Selected device: {:?}", device.name()
                                             .expect("Device has no name"));

    let format = device.default_output_format()
                       .expect("Failed to query default audio output format");
    println!("Selected audio format: {:?}", format);

    let my_stream = event_loop.build_output_stream(&device, &format)
                              .expect("Failed to create audio stream");

    // FIXME: Setup some kind of user input thread before running the event loop

    let mut playing = false;
    let mut elapsed_samples = 0;
    let sample_period = 1. / (format.sample_rate.0 as f32);
    event_loop.run(|stream_id, stream_result| {
        // Check our inputs
        assert_eq!(stream_id, my_stream, "Encountered unexpected stream");
        let stream_data =
            stream_result.expect("Something bad happened to the stream/device");

        // Access the output buffer
        use cpal::StreamData::Output;
        use cpal::UnknownTypeOutputBuffer::F32;
        if let Output { buffer: F32(mut buf) } = stream_data {
            for (sample_idx, sample) in buf.chunks_mut(format.channels as usize)
                                           .enumerate() {
                for (chan_idx, chan_out) in sample.iter_mut().enumerate() {
                    const MAGNITUDE : f32 = 0.05;
                    const SIN_PULS : f32 = 2.0 * PI * 440.0;
                    const DETUNE : f32 = 0.995;
                    let time = (elapsed_samples + sample_idx) as f32 * sample_period;
                    let chan_puls = SIN_PULS * (1.0 + (chan_idx as f32) * (DETUNE - 1.0));
                    *chan_out = MAGNITUDE * (chan_puls * time).sin();
                    *chan_out += 0.5 * MAGNITUDE * (2.0 * chan_puls * time).sin();
                    *chan_out += 0.25 * MAGNITUDE * (4.0 * chan_puls * time).sin();
                }
            }
            elapsed_samples += buf.len() / (format.channels as usize);
        } else {
            panic!("Unexpected stream format");
        }

        // We must feed the event loop at least once before starting playback
        if !playing {
            event_loop.play_stream(my_stream.clone())
                      .expect("Failed to start playback");
            playing = true;
        }
    });
}

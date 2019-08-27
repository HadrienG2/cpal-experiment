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

    // FIXME: Run the audio event loop in its own thread, and accept user input
    //        via stdin to adjust e.g. magnitude, detune...

    let mut playing = false;
    let mut elapsed_samples = 0;
    let sample_period = 1. / (format.sample_rate.0 as f32);
    event_loop.run(|stream_id, stream_result| {
        // Check our inputs
        assert_eq!(stream_id, my_stream, "Encountered unexpected stream");
        let stream_data =
            stream_result.expect("Something bad happened to the stream/device");

        // Fill the output buffer
        use cpal::StreamData::Output;
        use cpal::UnknownTypeOutputBuffer::F32;
        if let Output { buffer: F32(mut buf) } = stream_data {
            // For each sample...
            let num_chans = format.channels as usize;
            for (sample_idx, sample) in buf.chunks_mut(num_chans).enumerate() {
                // For each channel...
                for (chan_idx, chan_out) in sample.iter_mut().enumerate() {
                    // ...play the dumbest organ sound ever
                    const MAGNITUDE : f32 = 0.05;
                    const SIN_PULS : f32 = 2.0 * PI * 440.0;
                    const DETUNE : f32 = 0.995;
                    let time = (elapsed_samples + sample_idx) as f32 * sample_period;
                    let chan_puls = SIN_PULS * (1.0 + (chan_idx as f32) * (DETUNE - 1.0));
                    *chan_out = MAGNITUDE * (chan_puls * time).sin();
                    *chan_out += 0.5 * MAGNITUDE * (2.0 * chan_puls * time).sin();
                    *chan_out += 0.25 * MAGNITUDE * (3.0 * chan_puls * time).sin();
                    *chan_out += 0.125 * MAGNITUDE * (4.0 * chan_puls * time).sin();
                }
            }
            // Keep track of the passage of time
            elapsed_samples += buf.len() / num_chans;
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

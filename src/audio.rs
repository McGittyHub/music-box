use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use std::sync::{Arc, Mutex};

use crate::synth::Synth;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "full"))]
pub fn setup_audio(synth: &Arc<Mutex<Synth>>) {
    // Conditionally compile with jack if the feature is specified.
    #[cfg(all(
        any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd"),
        feature = "jack"
    ))]
    // Manually check for flags. Can be passed through cargo with -- e.g.
    // cargo run --release --example beep --features jack -- --jack
    let host = if std::env::args()
        .collect::<String>()
        .contains(&String::from("--jack"))
    {
        cpal::host_from_id(cpal::available_hosts()
            .into_iter()
            .find(|id| *id == cpal::HostId::Jack)
            .expect(
                "make sure --features jack is specified. only works on OSes where jack is available",
            )).expect("jack host unavailable")
    } else {
        cpal::default_host()
    };

    #[cfg(any(
        not(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd")),
        not(feature = "jack")
    ))]
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => setup_synth::<f32>(&device, &config.into(), synth).unwrap(),
        cpal::SampleFormat::U16 => setup_synth::<u16>(&device, &config.into(), synth).unwrap(),
        cpal::SampleFormat::I16 => setup_synth::<i16>(&device, &config.into(), synth).unwrap(),
    }
}

fn setup_synth<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    synth: &Arc<Mutex<Synth>>,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    // let synth = Arc::new(Mutex::new(Synth::new(sample_rate)));
    synth.lock().unwrap().sample_rate = sample_rate;

    let x = synth.clone();

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| write_data(data, channels, &x),
        err_fn,
    )?;
    stream.play()?;

    // TODO: Fix this
    std::mem::forget(stream); // Necessary, otherwise stream stop playing!

    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &Arc<Mutex<Synth>>)
where
    T: cpal::Sample,
{
    if let Ok(lock) = next_sample.lock().as_mut() {
        for frame in output.chunks_mut(channels) {
            let value: T = cpal::Sample::from::<f32>(&lock.next_sample());
            for sample in frame.iter_mut() {
                *sample = value;
            }
        }
    }
}

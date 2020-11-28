use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use wmidi::MidiMessage;

use std::{
    f32::consts::PI,
    sync::{Arc, Mutex},
};

use std::sync::mpsc;

use crate::midi::MidiEvent;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "full"))]
pub fn setup_audio(rx: mpsc::Receiver<MidiEvent>) {
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
        cpal::SampleFormat::F32 => setup_synth::<f32>(&device, &config.into(), rx).unwrap(),
        cpal::SampleFormat::I16 => setup_synth::<i16>(&device, &config.into(), rx).unwrap(),
        cpal::SampleFormat::U16 => setup_synth::<u16>(&device, &config.into(), rx).unwrap(),
    }
}

fn setup_synth<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: mpsc::Receiver<MidiEvent>,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    let mut keys_down = vec![];

    let mut sample_clock = 0f32;
    let next_value: Arc<Mutex<Box<dyn FnMut() -> f32 + Send + Sync>>> =
        Arc::new(Mutex::new(Box::new(move || 0.0)));

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let x = next_value.clone();

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| write_data(data, channels, &x),
        err_fn,
    )?;
    stream.play()?;

    let mut dirty = true;

    loop {
        if let Ok(e) = rx.try_recv() {
            match e.input {
                MidiMessage::NoteOff(_, n, _) => {
                    keys_down.retain(|&(x, _)| x != n);
                    dirty = true;
                }
                MidiMessage::NoteOn(_, n, v) => {
                    keys_down.retain(|&(x, _)| x != n);
                    keys_down.push((n, v));
                    dirty = true;
                }
                _ => {}
            }
        }

        if dirty {
            if let Ok(lock) = next_value.clone().try_lock().as_mut() {
                let keys = keys_down.clone();
                dirty = false;
                **lock = Box::new(move || {
                    sample_clock = (sample_clock + 1.0) % sample_rate;

                    let mut sample = 0.0;
                    for &(note, vel) in &keys {
                        let freq = note.to_freq_f32();
                        let vel = u8::from(vel) as f32 / 127.0;
                        sample += (sample_clock * freq * 2.0 * PI / sample_rate).sin() * vel;
                    }
                    sample
                });
            }
        }
    }
}

fn write_data<T>(
    output: &mut [T],
    channels: usize,
    next_sample: &Arc<Mutex<Box<dyn FnMut() -> f32 + Send + Sync>>>,
) where
    T: cpal::Sample,
{
    if let Ok(lock) = next_sample.lock().as_mut() {
        for frame in output.chunks_mut(channels) {
            let value: T = cpal::Sample::from::<f32>(&lock());
            for sample in frame.iter_mut() {
                *sample = value;
            }
        }
    }
}

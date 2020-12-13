extern crate anyhow;
extern crate cpal;
extern crate midir;
extern crate rustfft;

use std::{
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use std::sync::mpsc;

use audio::setup_audio;
use imgui::*;
use midi::{setup_midi, MidiEvent};
use rustfft::num_traits::Zero;
use rustfft::{num_complex::Complex, FFT};
use synth::Synth;
use ui::midi_drawer::draw_midi_viewer;
use wmidi::MidiMessage;

mod audio;
mod midi;
mod ringbuffer;
mod support;
mod synth;

mod ui;

fn main() {
    let mut conns = setup_midi().unwrap();

    let (tx, rx) = mpsc::channel::<MidiEvent>();

    let synth = Arc::new(Mutex::new(Synth::new(48_000.0)));

    let audio_synth = synth.clone();

    thread::spawn(move || {
        setup_audio(&audio_synth);

        loop {
            if let Ok(e) = rx.try_recv() {
                match e.input {
                    MidiMessage::NoteOff(_, n, _) => {
                        audio_synth.lock().unwrap().toggle_key_up(n);
                    }
                    MidiMessage::NoteOn(_, n, v) => {
                        let vel = u8::from(v) as f32 / 127.0;
                        audio_synth.lock().unwrap().toggle_key_down(n, vel);
                    }
                    _ => {}
                }
            }
        }
    });

    let system = support::init(file!());

    let mut notes = vec![];

    let mut current_time = 0;

    let mut last_tick = Instant::now();

    let mut frequencies = vec![0.0; 4096 * 16];
    let mut frequency_index = 0;

    let ui_synth = synth.clone();

    let mut last_samples = 0;

    let fft = rustfft::algorithm::Radix4::<f32>::new(frequencies.len(), false);

    let mut averages = vec![0.0; frequencies.len() / 2];

    system.main_loop(move |_, ui| {
        current_time += Instant::now().duration_since(last_tick).as_micros() as u64;
        last_tick = Instant::now();

        for conn in &mut conns {
            if let Ok(e) = conn.rx.try_recv() {
                conn.input.push(e.clone());
                notes.push(e.clone());
                current_time = e.time;
                tx.send(e).unwrap();
            }
        }

        if let Ok(synth) = ui_synth.lock() {
            let buffer = synth.sample_buffer();
            let samples = synth.samples();

            for s in 0..samples - last_samples {
                frequency_index = frequency_index % frequencies.len();
                frequencies[frequency_index] = buffer
                    .get((samples - last_samples - s - 1) as usize)
                    .cloned()
                    .unwrap(); // TODO: Is this a good idea?
                frequency_index += 1;
            }

            last_samples = samples;
        }

        let mut freqs = (0..frequencies.len())
            .map(|i| {
                Complex::new(
                    frequencies[(frequencies.len() - i + frequency_index) % frequencies.len()],
                    0.0,
                )
            })
            .collect::<Vec<_>>();

        let mut out = vec![Complex::zero(); freqs.len()];

        fft.process(&mut freqs, &mut out);

        out.truncate(freqs.len() / 2);

        for i in 0..averages.len() {
            averages[i] += (out[i].re.powi(2) + out[i].im.powi(2)).sqrt();
            averages[i] /= 32.0;
        }

        let midi_win_width = 800.0;
        let midi_win_height = 400.0;

        Window::new(im_str!("midi"))
            .position([0.0, 0.0], Condition::Always)
            .size([midi_win_width, midi_win_height], Condition::Always)
            .no_decoration()
            .build(ui, || {
                draw_midi_viewer(ui, &notes, current_time, midi_win_width, midi_win_height);
            });

        Window::new(im_str!("oscilloscope"))
            .position([0.0, midi_win_height], Condition::Always)
            .size([midi_win_width, midi_win_height], Condition::Always)
            .no_decoration()
            .build(ui, || {
                let draw_list = ui.get_window_draw_list();

                // Sliding oscilloscope
                // let freqs = (0..frequencies.len())
                //     .map(|i| {
                //         frequencies[(frequencies.len() - i + frequency_index) % frequencies.len()]
                //     })
                //     .collect::<Vec<_>>();

                for (i, f) in frequencies.windows(2).rev().enumerate() {
                    draw_list
                        .add_line(
                            [
                                i as f32 * midi_win_width / frequencies.len() as f32,
                                midi_win_height + (f[0] + 1.0) * midi_win_height / 2.0,
                            ],
                            [
                                (i + 1) as f32 * midi_win_width / frequencies.len() as f32,
                                midi_win_height + (f[1] + 1.0) * midi_win_height / 2.0,
                            ],
                            [1.0, 1.0, 1.0],
                        )
                        .build();
                }
            });

        Window::new(im_str!("spectrum"))
            .position([midi_win_width, midi_win_height], Condition::Always)
            .size([midi_win_width, midi_win_height], Condition::Always)
            .no_decoration()
            .build(ui, || {
                let draw_list = ui.get_window_draw_list();

                // let mut smoothed = vec![0.0; midi_win_width as usize];
                let mut smoothed = vec![0.0; averages.len() / 8];
                let downsampled = averages.len() / smoothed.len();
                for i in 0..smoothed.len() {
                    let slice = &averages[i.saturating_sub(downsampled / 2)
                        ..(i + downsampled / 2).min(smoothed.len())];
                    smoothed[i] = slice.iter().sum::<f32>() / (downsampled as f32 * 2.0);
                }

                let len = smoothed.len() as f32;

                let log_coef = 1.0 / (len + 1.0).log(std::f32::consts::E) * len;

                let displayed = (0..smoothed.len())
                    .map(|i| {
                        let f = len - (log_coef * (len + 1.0 - i as f32).log(std::f32::consts::E));
                        smoothed[f as usize] * (1.0 / len.sqrt())
                    })
                    .collect::<Vec<_>>();

                let max = displayed
                    .iter()
                    .cloned()
                    .max_by_key(|f| (f * 1000.0) as i32)
                    .unwrap();

                for (i, f) in displayed.windows(2).enumerate() {
                    let x = i as f32;

                    draw_list
                        .add_line(
                            [
                                midi_win_width + x * midi_win_width / len,
                                2.0 * midi_win_height - f[0] / max * midi_win_height,
                            ],
                            [
                                midi_win_width + (x + 1.0) * midi_win_width / len,
                                2.0 * midi_win_height - f[1] / max * midi_win_height,
                            ],
                            [1.0, 1.0, 1.0],
                        )
                        .build();
                }
            });

        Window::new(im_str!("synth"))
            .position([midi_win_width, 0.0], Condition::Always)
            .size([midi_win_width, midi_win_height], Condition::Always)
            .no_decoration()
            .build(ui, || {
                let draw_list = ui.get_window_draw_list();
                for (partial, &partial_volume) in synth.lock().unwrap().partials.iter().enumerate()
                {
                    draw_list
                        .add_rect(
                            [
                                midi_win_width + partial as f32 * midi_win_width / 64.0,
                                (1.0 - partial_volume) * midi_win_height,
                            ],
                            [
                                midi_win_width + (partial + 1) as f32 * midi_win_width / 64.0,
                                midi_win_height,
                            ],
                            [1.0, 1.0, 1.0],
                        )
                        .build();
                }
            });

        let [p_x, p_y] = ui.io().mouse_pos;
        if ui.is_mouse_down(MouseButton::Left)
            && p_x > midi_win_width
            && p_x < midi_win_width * 2.0
            && p_y > 0.0
            && p_y < midi_win_height
        {
            let partial = (64.0 * (p_x - midi_win_width) / midi_win_width) as usize;
            let p_vol = 1.0 - p_y / midi_win_height;

            synth.lock().unwrap().partials[partial] = p_vol;
        }
    });
}

extern crate anyhow;
extern crate cpal;
extern crate midir;

use std::{
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use std::sync::mpsc;

use audio::setup_audio;
use imgui::*;
use midi::{setup_midi, MidiEvent};
use synth::Synth;
use wmidi::{MidiMessage, Note};

mod audio;
mod midi;
mod support;
mod synth;

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

    let mut frequencies = vec![0.0; 1000];
    let mut frequency_index = 0;

    let ui_synth = synth.clone();

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

        frequency_index = frequency_index % frequencies.len();
        frequencies[frequency_index] = ui_synth.lock().unwrap().last_sample();
        frequency_index += 1;

        let midi_win_width = 1000.0;
        let midi_win_height = 400.0;

        Window::new(im_str!("midi"))
            .position([0.0, 0.0], Condition::Always)
            .size([midi_win_width, midi_win_height], Condition::Always)
            .no_decoration()
            .build(ui, || {
                let draw_list = ui.get_window_draw_list();

                let s_y = midi_win_height / 128.0;

                for i in 0..128 {
                    draw_list
                        .add_line(
                            [0.0, i as f32 * s_y],
                            [midi_win_width, (i + 1) as f32 * s_y],
                            [0.0, 0.0, 0.0],
                        )
                        .build();
                }

                if notes.len() >= 1 {
                    let start = notes[0].time as f32;
                    let end = current_time as f32;
                    let len = end - start as f32;

                    let mut notes_on = vec![];

                    let s_x = midi_win_width / len;

                    for note in &notes {
                        match note.input {
                            wmidi::MidiMessage::NoteOff(_, n, _) => {
                                let start_note = notes_on.swap_remove(
                                    notes_on
                                        .iter()
                                        .position(|i: &(u64, Note)| i.1 == n)
                                        .unwrap(),
                                );

                                let t1 = (start_note.0 as f32 - start) * s_x;
                                let t2 = (note.time as f32 - start) * s_x;

                                let n = u8::from(n) as f32;

                                draw_list
                                    .add_rect([t1, n * s_y], [t2, (n + 1.0) * s_y], [1.0, 1.0, 1.0])
                                    .filled(true)
                                    .build();
                            }
                            wmidi::MidiMessage::NoteOn(_, n, _) => {
                                notes_on.push((note.time, n));
                            }
                            _ => {}
                        }
                    }

                    for (time, note) in notes_on {
                        let t1 = (time as f32 - start) * s_x;
                        let t2 = (current_time as f32 - start) * s_x;

                        let n = u8::from(note) as f32;

                        draw_list
                            .add_rect([t1, n * s_y], [t2, (n + 1.0) * s_y], [1.0, 1.0, 1.0])
                            .filled(true)
                            .build();
                    }
                }
            });

        Window::new(im_str!("oscilloscope"))
            .position([0.0, midi_win_height], Condition::Always)
            .size([midi_win_width, midi_win_height], Condition::Always)
            .no_decoration()
            .build(ui, || {
                let draw_list = ui.get_window_draw_list();

                for (i, f) in frequencies.windows(2).enumerate() {
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
                        // .thickness(10.0)
                        .build();
                }
            });
    });
}

extern crate anyhow;
extern crate cpal;
extern crate midir;

use std::{thread, time::Instant};

use std::sync::mpsc;

use audio::setup_audio;
use imgui::*;
use midi::setup_midi;
use wmidi::Note;

mod audio;
mod midi;
mod support;

fn main() {
    let mut conns = setup_midi().unwrap();

    let (tx, rx) = mpsc::channel();

    thread::spawn(|| {
        setup_audio(rx);
    });

    let system = support::init(file!());

    let mut notes = vec![];

    let mut current_time = 0;

    let mut last_tick = Instant::now();

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

        let midi_win_width = 1000.0;
        let midi_win_height = 800.0;

        Window::new(im_str!("main"))
            .position([0.0, 0.0], Condition::Always)
            .size([midi_win_width, midi_win_height], Condition::Always)
            .no_decoration()
            // .always_auto_resize(true)
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
                        // .thickness(10.0)
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

        // for (i, con) in conns.iter().enumerate() {
        //     Window::new(&im_str!("{}", i))
        //         .size([300.0, 110.0], Condition::FirstUseEver)
        //         .build(ui, || {
        //             let event_names = con
        //                 .input
        //                 .iter()
        //                 .map(|e| im_str!("{} {:?}", e.time, e.input))
        //                 .collect::<Vec<_>>();

        //             let hack = event_names.iter().collect::<Vec<_>>();

        //             let mut idx = 0;
        //             ui.list_box(im_str!("input"), &mut idx, &hack, 32);
        //         });
        // }
    });
}

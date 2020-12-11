use imgui::Ui;
use wmidi::Note;

use crate::midi::MidiEvent;

pub fn draw_midi_viewer(ui: &Ui, notes: &[MidiEvent], current_time: u64, width: f32, height: f32) {
    if notes.len() >= 1 {
        let mut notes_on = vec![];

        let mut note_draw_list = vec![];

        let mut note_range = 255..0;

        for note in notes {
            match note.input {
                wmidi::MidiMessage::NoteOff(_, n, _) => {
                    let start_note = notes_on.swap_remove(
                        notes_on
                            .iter()
                            .position(|i: &(u64, Note)| i.1 == n)
                            .unwrap(),
                    );

                    note_draw_list.push((start_note.0, note.time, n));
                }
                wmidi::MidiMessage::NoteOn(_, n, _) => {
                    notes_on.push((note.time, n));

                    note_range.start = u8::from(n).min(note_range.start);
                    note_range.end = u8::from(n).max(note_range.end);
                }
                _ => {}
            }
        }

        let draw_list = ui.get_window_draw_list();

        let start = notes[0].time as f32;
        let end = current_time as f32;
        let len = end - start as f32;

        let displayed_note_range = note_range.clone().count() + 24;

        let s_x = width / len;
        let s_y = height / displayed_note_range as f32;

        for i in 0..displayed_note_range + 24 {
            draw_list
                .add_line(
                    [0.0, i as f32 * s_y],
                    [width, i as f32 * s_y],
                    [0.0, 0.0, 0.0],
                )
                .build();
        }

        for (time, note) in notes_on {
            note_draw_list.push((time, current_time, note));
        }

        for (t1, t2, note) in note_draw_list {
            let t1 = (t1 as f32 - start) * s_x;
            let t2 = (t2 as f32 - start) * s_x;

            let n = u8::from(note) as f32;

            draw_list
                .add_rect(
                    [t1, height - (n - note_range.start as f32 + 12.0) * s_y],
                    [
                        t2,
                        height - (n - note_range.start as f32 + 1.0 + 12.0) * s_y,
                    ],
                    [1.0, 1.0, 1.0],
                )
                .filled(true)
                .build();
        }
    }
}

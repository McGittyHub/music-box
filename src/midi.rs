use std::convert::TryFrom;

use std::{error::Error, sync::mpsc};

use midir::{Ignore, MidiInput, MidiInputConnection};
use wmidi::MidiMessage;

#[derive(Clone)]
pub struct MidiEvent {
    pub input: MidiMessage<'static>,
    pub time: u64,
}

pub struct MidiSource {
    _connection: MidiInputConnection<mpsc::Sender<MidiEvent>>,
    pub rx: mpsc::Receiver<MidiEvent>,
    _tx: mpsc::Sender<MidiEvent>,
    pub input: Vec<MidiEvent>,
}

pub fn setup_midi() -> Result<Vec<MidiSource>, Box<dyn Error>> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    let in_ports = midi_in.ports();

    let mut conns = vec![];

    for port in in_ports {
        let mut midi = MidiInput::new("")?;
        midi.ignore(Ignore::None);
        let (tx, rx) = mpsc::channel();
        conns.push(MidiSource {
            _connection: midi.connect(
                &port,
                "midir-read-input",
                move |stamp, bytes, tx| {
                    let message = wmidi::MidiMessage::try_from(bytes).unwrap().to_owned();

                    match message {
                        MidiMessage::NoteOff(_, _, _) => {}
                        MidiMessage::NoteOn(_, _, _) => {}
                        _ => {
                            return;
                        }
                    }

                    tx.send(MidiEvent {
                        input: message,
                        time: stamp,
                    })
                    .unwrap();
                },
                tx.clone(),
            )?,
            rx,
            _tx: tx,
            input: vec![],
        });
    }

    Ok(conns)
}

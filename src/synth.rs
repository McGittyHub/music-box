use std::f32::consts::PI;

use wmidi::Note;

struct Voice {
    key: Note,
    velocity: f32,
    time: f32,
}

pub struct Synth {
    pub sample_clock: f32,
    pub sample_rate: f32,
    time: f32,
    keys_pressed: Vec<Voice>,
}

struct ADSR {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

fn lin_lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

impl ADSR {
    pub fn evaluate(&self, t: f32, down: bool) -> f32 {
        if down {
            if t < self.attack {
                lin_lerp(0.0, 1.0, t / self.attack)
            } else if t < self.attack + self.decay {
                lin_lerp(1.0, self.sustain, t / (self.attack + self.decay))
            } else {
                self.sustain
            }
        } else {
            lin_lerp(1.0, 0.0, t / self.release)
        }
    }
}

impl Synth {
    pub fn new(sample_rate: f32) -> Self {
        Synth {
            sample_clock: 0.0,
            sample_rate,
            time: 0.0,
            keys_pressed: vec![],
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        self.sample_clock = (self.sample_clock + 1.0) % self.sample_rate;

        self.time += 1.0 / self.sample_rate;

        let adsr = ADSR {
            attack: 0.01,
            decay: 0.4,
            sustain: 0.5,
            release: 0.6,
        };

        let mut sample = 0.0;
        for voice in &self.keys_pressed {
            let freq = voice.key.to_freq_f32();
            let vol = adsr.evaluate(self.time - voice.time, true) * voice.velocity;
            sample += (self.sample_clock * freq * 2.0 * PI / self.sample_rate).sin() * vol;
        }
        sample
    }

    pub fn toggle_key_down(&mut self, key: Note, vel: f32) {
        self.keys_pressed.retain(|v| v.key != key);
        self.keys_pressed.push({
            Voice {
                key,
                velocity: vel,
                time: self.time,
            }
        });
    }

    pub fn toggle_key_up(&mut self, key: Note) {
        self.keys_pressed.retain(|v| v.key != key);
    }
}

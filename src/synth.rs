use std::f32::consts::PI;

use wmidi::Note;

use crate::ringbuffer::RingBuffer;

struct Voice {
    key: Note,
    velocity: f32,
    time: f32,
    release_time: Option<f32>,
}

pub struct Synth {
    pub sample_clock: f32,
    pub sample_rate: f32,
    time: f32,
    keys_pressed: Vec<Voice>,
    sample_buffer: RingBuffer<f32>,
    samples: u64,
}

struct ADSR {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

fn lin_lerp(a: f32, b: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t = 0.0;
    }
    if t > 1.0 {
        t = 0.0;
    }
    a * (1.0 - t) + b * t
}

impl ADSR {
    pub fn evaluate(&self, t: f32, t2: f32, down: bool) -> f32 {
        let v = if t < self.attack {
            lin_lerp(0.0, 1.0, t / self.attack)
        } else if t < self.attack + self.decay {
            lin_lerp(1.0, self.sustain, t / (self.attack + self.decay))
        } else {
            self.sustain
        };
        if down {
            v
        } else {
            lin_lerp(v, 0.0, t2 / self.release)
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
            sample_buffer: RingBuffer::with_size(4096),
            samples: 0,
        }
    }

    pub fn sample_buffer(&self) -> &RingBuffer<f32> {
        &self.sample_buffer
    }

    pub fn samples(&self) -> u64 {
        self.samples
    }

    pub fn next_sample(&mut self) -> f32 {
        // self.sample_clock = (self.sample_clock + 1.0) % self.sample_rate; // this produces clicks every second
        self.sample_clock += 1.0; // TODO: Fix, will go bad eventually

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

            let vol = if let Some(release_time) = voice.release_time {
                adsr.evaluate(self.time - voice.time, self.time - release_time, false)
                    * voice.velocity
            } else {
                adsr.evaluate(self.time - voice.time, 0.0, true) * voice.velocity
            };

            sample += (self.sample_clock * freq * 2.0 * PI / self.sample_rate).sin() * vol;
        }

        // Remove fully released voices
        let time = self.time;
        self.keys_pressed.retain(|v| {
            if let Some(release_time) = v.release_time {
                if time - release_time < adsr.release {
                    true
                } else {
                    false
                }
            } else {
                true
            }
        });

        self.sample_buffer.push(sample);
        self.samples += 1;

        sample
    }

    pub fn toggle_key_down(&mut self, key: Note, vel: f32) {
        self.keys_pressed.retain(|v| v.key != key);
        self.keys_pressed.push({
            Voice {
                key,
                velocity: vel,
                time: self.time,
                release_time: None,
            }
        });
    }

    pub fn toggle_key_up(&mut self, key: Note) {
        let v = self.keys_pressed.iter_mut().find(|v| v.key == key).unwrap();
        v.release_time = Some(self.time);
    }
}

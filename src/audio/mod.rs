use crate::apu::Apu;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapRb, traits::*};

pub struct Audio {
    abuf: ringbuf::HeapProd<f32>,
    _stream: cpal::Stream,
    passed_cycles: f64,
    cycles_per_sample: f64,
}

impl Audio {
    const CPU_FREQ: f64 = 1_789_773.0;

    pub fn new() -> Self {
        let (abuf, stream, cycles_per_sample) = Self::audio_initialize();
        Self { abuf, _stream: stream, passed_cycles: 0.0, cycles_per_sample }
    }

    fn audio_initialize() -> (ringbuf::HeapProd<f32>, cpal::Stream, f64) {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("No output device available");
        let config = device.default_output_config().expect("Failed to get default output config");
        let channels = config.channels() as usize;

        let ring = HeapRb::<f32>::new(8192);
        let (mut producer, mut consumer) = ring.split();

        for _ in 0..2048 {
            producer.try_push(0.0).ok();
        }

        let mut last_sample = 0.0;

        let stream = device.build_output_stream(&config.clone().into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let sample = consumer.try_pop().unwrap_or(last_sample);
                    last_sample = sample;
                    for ch in frame.iter_mut() {
                        *ch = sample;
                    }
                }
            }, |err| log::error!("Audio stream error: {}", err), None
        ).expect("Failed to build output stream");

        stream.play().expect("Failed to play stream");

        let rate = config.sample_rate();
        (producer, stream, Self::CPU_FREQ / rate as f64)
    }

    pub fn apu_steps(&mut self, apu: std::rc::Rc<std::cell::RefCell<Apu>>, cycles: usize) {
        for _ in 0..cycles {
            apu.borrow_mut().step_cycle();
            self.passed_cycles += 1.0;
            if self.passed_cycles >= self.cycles_per_sample {
                self.passed_cycles -= self.cycles_per_sample;
                let sample = apu.borrow_mut().output();
                loop {
                    if self.abuf.try_push(sample).is_ok() {
                        break;
                    }
                    std::thread::yield_now();
                }
            }
        }
    }
}

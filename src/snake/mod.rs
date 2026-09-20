use winit::event::*;
use winit::event_loop::ControlFlow;

use cfg_if::cfg_if;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::gui::GUI;
use crate::cpu::cpu::{Cpu, CpuFlags};
use crate::mmio::MmioInterface;
use triple_buffer::Input;

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

pub struct SnakeMmio {
    vmem: [u8; 0x10000]
}

impl SnakeMmio {
    pub fn new() -> Self {
        SnakeMmio { vmem: [0;0x10000] }
    }

    pub fn create_ptr() -> Box<dyn MmioInterface> {
        Box::new(SnakeMmio::new())
    }
}

impl MmioInterface for SnakeMmio {
    fn read(&self, addr: u16) -> u8
    {
        self.vmem[addr as usize]
    }
    fn read_range(&self, from: u16, to: u16, cbk: &mut dyn FnMut(usize, u8)) {
        let from = from as usize;
        let to = to as usize;
        self.vmem[from..to].iter().enumerate().for_each(|(i, b)| cbk(i, *b));
    }
    fn write(&mut self, addr: u16, data: u8)
    {
        self.vmem[addr as usize] = data;
    }
}

fn texture_format() -> wgpu::TextureFormat {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            wgpu::TextureFormat::Rgba8Unorm
        } else {
            wgpu::TextureFormat::Bgra8Unorm
        }
    }
}

// https://skilldrick.github.io/easy6502/
// https://gist.github.com/wkjagt/9043907
const SNAKE: &[u8] = &[
    0x20, 0x06, 0x06, 0x20, 0x38, 0x06, 0x20, 0x0d, 0x06, 0x20, 0x2a, 0x06, 0x60, 0xa9, 0x02, 0x85, // 0x0000
    0x02, 0xa9, 0x04, 0x85, 0x03, 0xa9, 0x11, 0x85, 0x10, 0xa9, 0x10, 0x85, 0x12, 0xa9, 0x0f, 0x85, // 0x0010
    0x14, 0xa9, 0x04, 0x85, 0x11, 0x85, 0x13, 0x85, 0x15, 0x60, 0xa5, 0xfe, 0x85, 0x00, 0xa5, 0xfe, // 0x0020
    0x29, 0x03, 0x18, 0x69, 0x02, 0x85, 0x01, 0x60, 0x20, 0x4d, 0x06, 0x20, 0x8d, 0x06, 0x20, 0xc3, // 0x0030
    0x06, 0x20, 0x19, 0x07, 0x20, 0x20, 0x07, 0x20, 0x2d, 0x07, 0x4c, 0x38, 0x06, 0xa5, 0xff, 0xc9, // 0x0040
    0x77, 0xf0, 0x0d, 0xc9, 0x64, 0xf0, 0x14, 0xc9, 0x73, 0xf0, 0x1b, 0xc9, 0x61, 0xf0, 0x22, 0x60, // 0x0050
    0xa9, 0x04, 0x24, 0x02, 0xd0, 0x26, 0xa9, 0x01, 0x85, 0x02, 0x60, 0xa9, 0x08, 0x24, 0x02, 0xd0, // 0x0060
    0x1b, 0xa9, 0x02, 0x85, 0x02, 0x60, 0xa9, 0x01, 0x24, 0x02, 0xd0, 0x10, 0xa9, 0x04, 0x85, 0x02, // 0x0070
    0x60, 0xa9, 0x02, 0x24, 0x02, 0xd0, 0x05, 0xa9, 0x08, 0x85, 0x02, 0x60, 0x60, 0x20, 0x94, 0x06, // 0x0080
    0x20, 0xa8, 0x06, 0x60, 0xa5, 0x00, 0xc5, 0x10, 0xd0, 0x0d, 0xa5, 0x01, 0xc5, 0x11, 0xd0, 0x07, // 0x0090
    0xe6, 0x03, 0xe6, 0x03, 0x20, 0x2a, 0x06, 0x60, 0xa2, 0x02, 0xb5, 0x10, 0xc5, 0x10, 0xd0, 0x06, // 0x00A0
    0xb5, 0x11, 0xc5, 0x11, 0xf0, 0x09, 0xe8, 0xe8, 0xe4, 0x03, 0xf0, 0x06, 0x4c, 0xaa, 0x06, 0x4c, // 0x00B0
    0x35, 0x07, 0x60, 0xa6, 0x03, 0xca, 0x8a, 0xb5, 0x10, 0x95, 0x12, 0xca, 0x10, 0xf9, 0xa5, 0x02, // 0x00C0
    0x4a, 0xb0, 0x09, 0x4a, 0xb0, 0x19, 0x4a, 0xb0, 0x1f, 0x4a, 0xb0, 0x2f, 0xa5, 0x10, 0x38, 0xe9, // 0x00D0
    0x20, 0x85, 0x10, 0x90, 0x01, 0x60, 0xc6, 0x11, 0xa9, 0x01, 0xc5, 0x11, 0xf0, 0x28, 0x60, 0xe6, // 0x00E0
    0x10, 0xa9, 0x1f, 0x24, 0x10, 0xf0, 0x1f, 0x60, 0xa5, 0x10, 0x18, 0x69, 0x20, 0x85, 0x10, 0xb0, // 0x00F0
    0x01, 0x60, 0xe6, 0x11, 0xa9, 0x06, 0xc5, 0x11, 0xf0, 0x0c, 0x60, 0xc6, 0x10, 0xa5, 0x10, 0x29, // 0x0100
    0x1f, 0xc9, 0x1f, 0xf0, 0x01, 0x60, 0x4c, 0x35, 0x07, 0xa0, 0x00, 0xa5, 0xfe, 0x91, 0x00, 0x60, // 0x0110
    0xa6, 0x03, 0xa9, 0x00, 0x81, 0x10, 0xa2, 0x00, 0xa9, 0x01, 0x81, 0x10, 0x60, 0xa2, 0x00, 0xea, // 0x0120
    0xea, 0xca, 0xd0, 0xfb, 0x60,                                                                   // 0x0130
];


fn decode_color(b: u8) -> wgpu::Color {
    use wgpu::Color;
    match b {
        0 => Color::BLACK,
        1 => Color::WHITE,
        2 | 9 => Color{ r: 0.5, g: 0.5, b: 0.5, a: 1.0 },
        3 | 10 => Color::RED,
        4 | 11 => Color::GREEN,
        5 | 12 => Color::BLUE,
        6 | 13 => Color{ r: 1.0, g: 0.0, b: 1.0, a: 1.0 },
        7 | 14 => Color{ r: 1.0, g: 1.0, b: 0.0, a: 1.0 },
        _ => Color{ r: 0.0, g: 1.0, b: 1.0, a: 1.0 },
    }
}

fn encode_input(key: winit::event::VirtualKeyCode) -> Option<u8> {
    match key {
        VirtualKeyCode::W => Some(0x77),
        VirtualKeyCode::A => Some(0x61),
        VirtualKeyCode::S => Some(0x73),
        VirtualKeyCode::D => Some(0x64),
        _ => None,
    }
}

pub fn run() {
    let gui = GUI::<32, 32>::new("Snake", 
        winit::dpi::LogicalSize::new(600, 600), 
        texture_format());

    let input = Arc::new(AtomicU8::new(0));


    let input_for_thread = input.clone(); 
    let nes_loop = move |inbuf: &mut Input<Vec<u8>>| {
        let mut rng = StdRng::from_entropy();
        let mut cpu = Cpu::with_bus(SnakeMmio::create_ptr());
        
        let start: u16 = 0x0600;
        let end = cpu.load_program(start, &SNAKE.to_vec());

        cpu.reset();

        cpu.sp = 0xff;
        cpu.flags = CpuFlags::from_bits_truncate(0b00110000);
        log::info!("CPU start state - {}", cpu);

        loop {
            log::debug!("CPU {}", cpu);
            cpu.step();
            if start >= cpu.pc || cpu.pc >= end {
                break true;
            }

            cpu.write(0xfe, rng.gen_range(1..16) as u8);

            let mut data = [0u8; (32 * 32 * 4) as usize];
            cpu.bus.read_range(0x200, 0x600, &mut |i, b| {
                let color = decode_color(b);

                let idx = i * 4;
                data[idx..idx+4].copy_from_slice(&[
                    (color.b * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.r * 255.0) as u8,
                    255,
                ]);
            });

            cpu.write(0xff, input_for_thread.load(Ordering::Relaxed));
            inbuf.write(data.to_vec());
            std::thread::sleep(std::time::Duration::from_micros(150));
        }
    };

    let input_for_handler = input.clone(); 
    let control_handler = move |input: KeyboardInput|-> ControlFlow {
        let mut ret = ControlFlow::Poll;
        if let Some(keycode) = input.virtual_keycode {
            match input.state {
                ElementState::Pressed => {
                    match keycode {
                        VirtualKeyCode::Escape => ret = ControlFlow::Exit,
                        VirtualKeyCode::W | VirtualKeyCode::A | VirtualKeyCode::S | VirtualKeyCode::D |
                        VirtualKeyCode::H | VirtualKeyCode::J | VirtualKeyCode::K | VirtualKeyCode::L |
                        VirtualKeyCode::Space | VirtualKeyCode::Return => {
                            if let Some(code) = encode_input(keycode) {
                                log::info!("Write input code: {} ({:?})", code, keycode);
                                input_for_handler.store(code, Ordering::Relaxed);
                            }
                        },
                        _ => {},
                    }
                },
                ElementState::Released => {}
            }
        }
        ret
    };

    gui.run(nes_loop, control_handler);
}

use capynes_lib::logger;
use capynes_lib::gui::GUI;
use capynes_lib::rom::Rom;
use capynes_lib::ppu::Ppu;
use capynes_lib::apu::Apu;
use capynes_lib::mmio::NesMmio;
use capynes_lib::cpu::cpu::Cpu;
use capynes_lib::ctrl::{Controller, Buttons, create_controller_state, press, release};
use capynes_lib::audio::Audio;

use winit::event::*;
use winit::event_loop::ControlFlow;

use triple_buffer::Input;

const DEFAULT_ROM: &str = "./test_rom/smb.nes";

struct InterruptEdges {
    first_nmi: Option<usize>,
    first_irq: Option<usize>,
}

fn run(rom_path: String) {
    let gui = GUI::<256, 240>::new("Capynes", 
        winit::dpi::LogicalSize::new(512, 480), 
        wgpu::TextureFormat::Bgra8Unorm);

    let js1 = create_controller_state();
    let js2 = create_controller_state();

    let j1 = js1.clone();
    let j2 = js2.clone();
    let nes_loop = move |inbuf: &mut Input<Vec<u8>>| {
        let rom = Rom::new(&rom_path).expect("Cannot load rom");
        log::info!("{}", rom);
        let apu = Apu::new(rom.clone());
        let ppu = Ppu::new(rom.clone());
        let j1 = Controller::new(j1.clone());
        let j2 = Controller::new(j2.clone());
        let mmio = NesMmio::new(rom, apu.clone(), ppu.clone(), j1, j2);
        let mut cpu = Cpu::with_bus(mmio);
        cpu.reset();

        let mut audio = Audio::new();
        let mut pending_nmi = false;

        loop {
            let cycles = cpu.step();
            let edges = advance_components(&mut cpu, &ppu, &apu, &mut audio, cycles);
            let mut fresh_nmi = edges.first_nmi;
            if let Some(cpu_cycle) = fresh_nmi {
                if cpu.last_opcode == 0x00 && cpu_cycle < 5 {
                    cpu.hijack_interrupt_vector_to_nmi();
                    fresh_nmi = None;
                }
            }

            let irq_became_pending_on_last_cycle =
                edges.first_irq.is_some_and(|cpu_cycle| cpu_cycle + 1 >= cycles);

            if cpu.bus.poll_irq()
                && !cpu.irq_inhibited()
                && !irq_became_pending_on_last_cycle
            {
                let interrupt_steps = cpu.irq_interrupt();
                let irq_edges = advance_components(&mut cpu, &ppu, &apu, &mut audio, interrupt_steps);
                let irq_fresh_nmi = irq_edges.first_nmi;

                if let Some(cpu_cycle) = irq_fresh_nmi {
                    if cpu_cycle < 5 {
                        cpu.hijack_interrupt_vector_to_nmi();
                    } else if fresh_nmi.is_none() {
                        fresh_nmi = Some(cpu_cycle);
                    }
                }
            }

            if pending_nmi {
                let interrupt_steps = cpu.nmi_interrupt();
                pending_nmi = false;
                let _ = advance_components(&mut cpu, &ppu, &apu, &mut audio, interrupt_steps);
            }

            if fresh_nmi.is_some() {
                ppu.borrow().frame().render_to(inbuf);
                pending_nmi = true;
            }

            cpu.finish_interrupt_poll();
        }
    };

    let j1 = js1.clone();
    let control_handler = move |input: KeyboardInput|-> ControlFlow {
        let mut ret = ControlFlow::Poll;
        if let Some(keycode) = input.virtual_keycode {
            match input.state {
                ElementState::Pressed => {
                    match keycode {
                        VirtualKeyCode::W => press(&j1, Buttons::UP), 
                        VirtualKeyCode::A => press(&j1, Buttons::LEFT), 
                        VirtualKeyCode::S => press(&j1, Buttons::DOWN), 
                        VirtualKeyCode::D => press(&j1, Buttons::RIGHT), 
                        VirtualKeyCode::J => press(&j1, Buttons::A), 
                        VirtualKeyCode::K => press(&j1, Buttons::B), 
                        VirtualKeyCode::T => press(&j1, Buttons::SELECT), 
                        VirtualKeyCode::Y => press(&j1, Buttons::START), 
                        VirtualKeyCode::Escape => ret = ControlFlow::Exit,
                        _ => {},
                    }
                    log::trace!("Pressed: {:?}", keycode);
                },
                ElementState::Released => {
                    match keycode {
                        VirtualKeyCode::W => release(&j1, Buttons::UP), 
                        VirtualKeyCode::A => release(&j1, Buttons::LEFT), 
                        VirtualKeyCode::S => release(&j1, Buttons::DOWN), 
                        VirtualKeyCode::D => release(&j1, Buttons::RIGHT), 
                        VirtualKeyCode::J => release(&j1, Buttons::A), 
                        VirtualKeyCode::K => release(&j1, Buttons::B), 
                        VirtualKeyCode::T => release(&j1, Buttons::SELECT), 
                        VirtualKeyCode::Y => release(&j1, Buttons::START), 
                        _ => {},
                    }
                    log::trace!("Released: {:?}", keycode);
                }
            }
        }
        ret
    };

    gui.run(nes_loop, control_handler);
}

fn advance_components(
    cpu: &mut Cpu,
    ppu: &std::rc::Rc<std::cell::RefCell<Ppu>>,
    apu: &std::rc::Rc<std::cell::RefCell<Apu>>,
    audio: &mut Audio,
    cpu_cycles: usize,
) -> InterruptEdges {
    let mut first_nmi = None;
    let mut first_irq = None;

    for cpu_cycle in 0..cpu_cycles {
        ppu.borrow_mut().step_cycles(3);
        audio.apu_steps(apu.clone(), 1);

        if first_nmi.is_none() && ppu.borrow_mut().poll_nmi() {
            first_nmi = Some(cpu_cycle);
        }

        if first_irq.is_none() && cpu.bus.poll_irq() {
            first_irq = Some(cpu_cycle);
        }
    }

    InterruptEdges {
        first_nmi,
        first_irq,
    }
}

fn main() {
    logger::init_logging();
    log::info!("+ Application started");

    let rom_path = match std::env::args_os().nth(1) {
        Some(arg) => {
            let arg = arg.to_string_lossy();
            if arg == "-h" || arg == "--help" {
                eprintln!("Usage: capynes [ROM_PATH]");
                eprintln!("Default ROM: {}", DEFAULT_ROM);
                return;
            }
            arg.into_owned()
        }
        None => DEFAULT_ROM.to_string(),
    };

    run(rom_path);
}

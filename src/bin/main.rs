pub mod chip {

    pub struct CPU {
        pub PC: u32,
        pub I: u32,
        pub V: [u8; 16],
        pub SP: u32,
        pub instr: u16,
    }

    impl CPU {
        pub fn new() -> Self {
            CPU {
                PC: 0xAFu32,
                I: 0x23u32,
                V: [10, 11, 12, 16, 17, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1],
                SP: 0x23u32,
                instr: 0x00E0,
            }
        }
    }

    pub struct Emulator {
        pub cpu: CPU,
    }
}

mod render {
    use crate::chip::*;
    use easycurses::Color::*;
    use easycurses::*;

    fn render_line(e: &mut EasyCurses, cp: ColorPair, s: String) {
        e.set_color_pair(cp);
        e.print(s);
    }

    fn part(e: &mut EasyCurses, label: &str, val: u32) {
        render_line(e, colorpair!(Yellow on Black), format!("0x{:02X} ", val));
        render_line(
            e,
            colorpair!(Green on Black),
            format!("{label:10} ", label = label),
        );
    }

    fn pc(e: &mut EasyCurses, r: i32, c: i32, cpu: &CPU) {
        let mut r = r;
        e.move_rc(r, c);
        part(e, "PC", cpu.PC);

        r += 1;

        e.move_rc(r, c);
        part(e, "I", cpu.I);

        r += 1;
        e.move_rc(r, c);
        part(e, "SP", cpu.SP);

        r += 4;
        for i in 0..0x10 {
            r += 1;
            e.move_rc(r, c);
            part(e, &format!("V{:1X}", i), cpu.V[i as usize].into());
        }

        r += 2;
        e.move_rc(r, c);
        part(e, "instruction", cpu.instr.into());
    }

    pub fn chip(ch: &Emulator) {
        if let Some(mut e) = EasyCurses::initialize_system() {
            e.set_cursor_visibility(CursorVisibility::Invisible);
            e.set_echo(false);
            pc(&mut e, 3, 5, &ch.cpu);
            e.refresh();
            e.get_input();
        }
    }
}

fn main() {
    let c = chip::Emulator {
        cpu: chip::CPU::new(),
    };
    render::chip(&c);
}

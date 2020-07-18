mod render {

    use libchip8::cpu;
    use libchip8::display;
    use libchip8::emulator;
    use libchip8::input;

    use easycurses::Color::*;
    use easycurses::*;

    pub struct Config {
        present: char,
        absent: char,
        color_present: ColorPair,
        color_absent: ColorPair,
        display_width: i32,
        display_height: i32,
    }

    impl Config {
        const MAPPING: &'static str = "x123qweasdzc4rfv";
        const ORIG: &'static str = "123C456D789EA0BF";

        fn new(
            present: char,
            absent: char,
            color_present: ColorPair,
            color_absent: ColorPair,
            dw: i32,
            dh: i32,
        ) -> Config {
            Config {
                present,
                absent,
                color_present,
                color_absent,
                display_width: dw,
                display_height: dh,
            }
        }

        fn map_base16_to_key(idx: usize) -> Option<char> {
            Config::MAPPING.chars().nth(idx)
        }

        fn map_key_to_base16(k: char) -> Option<usize> {
            Config::MAPPING
                .char_indices()
                .find(|(_, c)| *c == k)
                .map(|(idx, _)| idx)
        }
    }

    fn render_line(e: &mut EasyCurses, cp: ColorPair, s: String) {
        e.set_color_pair(cp);
        e.print(s);
    }

    fn part(e: &mut EasyCurses, label: &str, val: u16) {
        part_str(e, label, format!("0x{:04X} ", val));
    }

    fn render_labelled(e: &mut EasyCurses, label: &str, s: &str, max_width: usize) {
        let lab = format!("{label:10} ", label = label);
        let lablen = lab.len();
        render_line(e, colorpair!(Green on Black), lab);

        let mut s = String::from(s);
        s.push_str(&" ".repeat(std::cmp::max(0, max_width - (s.len() + lablen))));
        render_line(e, colorpair!(Yellow on Black), s);
    }

    fn part_str(e: &mut EasyCurses, label: &str, s: String) {
        render_line(
            e,
            colorpair!(Green on Black),
            format!("{label:10} ", label = label),
        );
        render_line(e, colorpair!(Yellow on Black), s);
    }

    fn render_kbd_line(
        e: &mut EasyCurses,
        r: i32,
        c: i32,
        nums: [i8; 4],
        kbd: &input::Keyboard,
        cfg: &Config,
    ) {
        for i in 0..4 {
            let num = nums[i as usize] as usize;
            e.move_rc(r, c + i * 6);
            e.set_color_pair(if kbd.get(num as usize) {
                cfg.color_present
            } else {
                cfg.color_absent
            });

            let key = format!("[{:X}]{} ", num, Config::map_base16_to_key(num).unwrap());
            e.print(key);
        }
    }

    fn render_keyboard(e: &mut EasyCurses, r: i32, c: i32, kbd: &input::Keyboard, cfg: &Config) {
        render_kbd_line(e, r, c, [1, 2, 3, 0xC], kbd, cfg);
        render_kbd_line(e, r + 1, c, [4, 5, 6, 0xD], kbd, cfg);
        render_kbd_line(e, r + 2, c, [7, 8, 9, 0xE], kbd, cfg);
        render_kbd_line(e, r + 3, c, [0xA, 0, 0xB, 0xF], kbd, cfg);
    }

    fn render_cpu(e: &mut EasyCurses, r: i32, c: i32, max_width: usize, cpu: &cpu::CPU) {
        let mut r = r;
        e.move_rc(r, c);
        part(e, "PC", cpu.pc);

        r += 1;

        e.move_rc(r, c);
        part(e, "I", cpu.i);

        r += 1;
        e.move_rc(r, c);
        part(e, "SP", cpu.sp);

        r += 4;
        for i in 0..0x10 {
            r += 1;
            e.move_rc(r, c);
            part(e, &format!("V{:01X}", i), cpu.regs[i as usize].into());
        }

        r += 2;
        e.move_rc(r, c);
        part(
            e,
            "instruction",
            if let Some(ref op) = cpu.instr {
                op.to_instr()
            } else {
                0
            },
        );
        e.move_rc(r + 1, c);
        render_labelled(e, "opcode", &format!("{:?}", cpu.instr), max_width);
    }

    fn render_frame(e: &mut EasyCurses, r: i32, c: i32, conf: &Config) {
        let wi = conf.display_width;
        let hi = conf.display_height;

        e.move_rc(r, c);
        e.print_char('<');
        for i in 0..wi {
            e.move_rc(r, c + 1 + i);
            e.print_char('-');
        }
        e.move_rc(r, c + wi + 1);
        e.print_char('>');

        for i in 0..hi {
            e.move_rc(r + 1 + i, c);
            //e.print_char('│');
            e.print_char('|');
            e.move_rc(r + 1 + i, c + wi + 1);
            e.print_char('|');
        }

        e.move_rc(r + hi + 1, c);
        e.print_char('<');
        for i in 0..wi {
            e.move_rc(r + hi + 1, c + 1 + i);
            e.print_char('-');
        }
        e.move_rc(r + hi + 1, c + wi + 1);
        e.print_char('>');
    }

    fn render_display(e: &mut EasyCurses, r: i32, c: i32, d: &display::Screen, cfg: &Config) {
        for y in 0..display::ROWS {
            for x in 0..display::COLS {
                let bit = d.get(x as u8, y as u8);
                //let bit = (x + y) > 10 && (x + y) < 50;
                let z = if bit { cfg.present } else { cfg.absent };
                let row = r + y as i32;
                let col = c + x as i32;
                e.set_color_pair(if bit {
                    cfg.color_present
                } else {
                    cfg.color_absent
                });
                e.move_rc(row, col);
                e.print_char(z);
            }
        }
    }

    fn render_step(e: &mut EasyCurses, r: i32, c: i32, step: i32) {
        e.move_rc(r, c);
        e.print(step.to_string());
    }

    pub fn chip_loop(ch: &mut emulator::Emulator, c: &Config) {
        let mut step_count = 0;
        let mut oldk: Option<usize> = None;
        let x0 = 10;
        let y0 = 1;
        let cpu_width = 40;
        if let Some(mut e) = EasyCurses::initialize_system() {
            e.set_cursor_visibility(CursorVisibility::Invisible);
            e.set_echo(false);
            e.set_input_mode(InputMode::Character);
            render_frame(&mut e, y0, x0 + cpu_width, c);
            loop {
                let (r, _) = e.get_row_col_count();
                render_cpu(&mut e, y0, x0, cpu_width as usize, &(*ch).cpu);
                render_display(&mut e, y0 + 1, x0 + cpu_width + 1, &(*ch).scr, c);
                render_keyboard(&mut e, r - 2 - 5, x0, &ch.kbd, c);
                render_step(&mut e, r - 2, x0, step_count);

                e.refresh();

                if let Some(ip) = e.get_input() {
                    step_count += 1;
                    match ip {
                        Input::Character(',') => break,
                        Input::Character(key) => {
                            if let Some(newk) = Config::map_key_to_base16(key) {
                                ch.key_pressed(oldk, newk);
                                oldk = Some(newk);
                            }
                        }
                        _ => (),
                    }
                    ch.step();
                }
            }
        }
    }

    pub fn default_config() -> Config {
        Config::new(
            '*',
            '_',
            ColorPair::new(Yellow, Black),
            ColorPair::new(Blue, Black),
            64,
            32,
        )
    }
}

mod run {
    use std::thread::sleep;
    use std::time::Duration;
    use std::time::Instant;

    use crate::render;
    use libchip8::emulator;
    use libchip8::loader;

    pub fn emulation(ch: &mut emulator::Emulator, fname: &str) {
        loader::load(ch, &String::from(fname));
        ch.store_font();
        render::chip_loop(ch, &render::default_config());
    }
}
use libchip8::emulator;
use std::env;
fn main() {
    let args: Vec<String> = env::args().collect();
    let fname = &args[1];
    let mut c = emulator::Emulator::new();
    run::emulation(&mut c, fname);
}

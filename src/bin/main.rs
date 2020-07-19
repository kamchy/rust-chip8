mod render {

    use libchip8::cpu;
    use libchip8::display;
    use libchip8::emulator;
    use libchip8::input;

    use easycurses::Color::*;
    use easycurses::*;

    use std::thread::sleep;
    use std::time::Duration;
    use std::time::Instant;

    /// Maps 16-keys chip-8 keyboard to contemporary keyboard layout
    mod KeyMap {
        /// i-th character represents key which - when pressed - is mapped to key i in chip-8 kbd
        const MAPPING: &'static str = "x123qweasdzc4rfv";

        pub(crate) fn map_base16_to_key(idx: usize) -> Option<char> {
            MAPPING.chars().nth(idx)
        }

        pub(crate) fn map_key_to_base16(k: char) -> Option<usize> {
            MAPPING
                .char_indices()
                .find(|(_, c)| *c == k)
                .map(|(idx, _)| idx)
        }
    }

    pub struct Config {
        present: char,
        absent: char,
        color_present: ColorPair,
        color_absent: ColorPair,
        display_width: i32,
        display_height: i32,
        x0: i32,
        y0: i32,
        cpu_width: i32,
        cpu_height: i32,
        kbd_height: i32,
    }

    impl Config {
        fn display_position(&self) -> (i32, i32) {
            let r = self.y0 + 1;
            let c = self.x0 + self.cpu_width + 2;
            (r, c)
        }

        fn frame_position(&self) -> (i32, i32) {
            let r = self.y0;
            let c = self.x0 + self.cpu_width + 1;
            (r, c)
        }

        fn keyboard_position(&self) -> (i32, i32) {
            let r = self.y0 + self.cpu_height + 1;
            let c = self.x0;
            (r, c)
        }

        fn step_position(&self) -> (i32, i32) {
            let r = self.y0 + self.cpu_height + self.kbd_height + 2;
            let c = self.x0;
            (r, c)
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

            let key = format!("[{:X}]{} ", num, KeyMap::map_base16_to_key(num).unwrap());
            e.print(key);
        }
    }

    fn render_keyboard(e: &mut EasyCurses, kbd: &input::Keyboard, cfg: &Config) {
        let (r, c) = cfg.keyboard_position();
        render_kbd_line(e, r, c, [1, 2, 3, 0xC], kbd, cfg);
        render_kbd_line(e, r + 1, c, [4, 5, 6, 0xD], kbd, cfg);
        render_kbd_line(e, r + 2, c, [7, 8, 9, 0xE], kbd, cfg);
        render_kbd_line(e, r + 3, c, [0xA, 0, 0xB, 0xF], kbd, cfg);
    }

    fn render_cpu(e: &mut EasyCurses, cpu: &cpu::CPU, cfg: &Config) {
        let mut r = cfg.y0;
        let c = cfg.x0;

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
        render_labelled(
            e,
            "opcode",
            &format!("{:?}", cpu.instr),
            cfg.cpu_width as usize,
        );
    }

    fn render_frame(e: &mut EasyCurses, conf: &Config) {
        let (r, c) = conf.frame_position();
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

    fn render_display(e: &mut EasyCurses, d: &display::Screen, cfg: &Config) {
        let (r, c) = cfg.display_position();
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

    fn render_step(e: &mut EasyCurses, step: i32, cfg: &Config) {
        let (r, c) = cfg.step_position();
        e.move_rc(r, c);
        e.print(step.to_string());
    }

    pub fn chip_loop(ch: &mut emulator::Emulator, c: &Config) {
        let mut step_count = 0;
        let mut oldk: Option<usize> = None;
        let mut next_instr = ch.fetch();

        let frame_target_duration = Duration::new(1, 0)
            .checked_div(60)
            .expect("failed when rhs!=0, what?");

        if let Some(mut e) = EasyCurses::initialize_system() {
            e.set_cursor_visibility(CursorVisibility::Invisible);
            e.set_echo(false);
            e.set_input_mode(InputMode::Character);
            render_frame(&mut e, c);

            loop {
                let top_of_loop = Instant::now();
                render_cpu(&mut e, &(*ch).cpu, c);
                render_display(&mut e, &(*ch).scr, c);
                render_keyboard(&mut e, &ch.kbd, c);
                render_step(&mut e, step_count, c);

                e.refresh();

                if let Some(ip) = e.get_input() {
                    step_count += 1;
                    match ip {
                        Input::Character(',') => break,
                        Input::Character(key) => {
                            if let Some(newk) = KeyMap::map_key_to_base16(key) {
                                ch.key_pressed(oldk, newk);
                                oldk = Some(newk);
                            }
                        }
                        _ => (),
                    }
                    //ch.step();
                    if let Some(instr) = next_instr {
                        ch.exec(instr);
                        next_instr = ch.fetch();
                    }
                }

                let elapsed_this_frame = top_of_loop.elapsed();
                if let Some(frame_remaining) = frame_target_duration.checked_sub(elapsed_this_frame)
                {
                    sleep(frame_remaining);
                }
            }
        }
    }

    pub fn default_config() -> Config {
        Config {
            present: '*',
            absent: ' ',
            color_present: ColorPair::new(Yellow, Black),
            color_absent: ColorPair::new(Blue, Black),
            display_width: 64,
            display_height: 32,
            x0: 3,
            y0: 3,
            cpu_width: 40,
            cpu_height: 25,
            kbd_height: 5,
        }
    }
}

mod run {

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

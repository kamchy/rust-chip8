/// Maps 16-keys chip-8 keyboard to contemporary keyboard layout
mod key_map {
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

mod render {

    use libchip8::cpu;
    use libchip8::display;
    use libchip8::emulator;
    use libchip8::input;

    use easycurses::Color::*;
    use easycurses::*;

    use crate::key_map;
    use crate::mode::RunMode;

    use std::thread::sleep;
    use std::time::Duration;
    use std::time::Instant;

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

    /// Defines a set of xxx_position()
    /// functions that retur (row, col) tuples
    impl Config {
        fn display_position(&self) -> (i32, i32) {
            let (r, c) = self.frame_position();
            (r + 1, c + 1)
        }

        fn cpu_position(&self) -> (i32, i32) {
            (self.y0, self.x0)
        }

        fn frame_position(&self) -> (i32, i32) {
            let (r, c) = self.cpu_position();
            (r, c + self.cpu_width + 7)
        }

        fn keyboard_position(&self) -> (i32, i32) {
            let (r, c) = self.cpu_position();
            (r + self.cpu_height, c)
        }

        fn step_position(&self) -> (i32, i32) {
            let (r, c) = self.keyboard_position();
            (r + self.kbd_height, c)
        }

        fn dt_st_position(&self) -> (i32, i32) {
            let (r, c) = self.step_position();
            (r + 1, c)
        }
        fn status_position(&self) -> (i32, i32) {
            let (r, c) = self.dt_st_position();
            (r + 1, c)
        }
    }

    pub trait Renderer {
        fn render_cpu(&mut self, cpu: &cpu::CPU);
        fn render_display(&mut self, d: &dyn display::Scr);
        fn render_keyboard(&mut self, kbd: &input::Keyboard);
        fn render_dt_st(&mut self, dt_st: (u8, u8));
        fn render_status(&mut self, s: &str);
        fn render_frame(&mut self);
        fn render_step(&mut self, step: u64, fps: u64);
        fn wait_to_quit(&mut self, s: &str);
    }

    pub struct EasyCursesRenderer<'s> {
        e: &'s mut EasyCurses,
        cfg: &'s Config,
    }

    impl<'s> EasyCursesRenderer<'s> {
        fn new(e: &'s mut EasyCurses, cfg: &'s Config, rm: RunMode) -> Self {
            e.set_cursor_visibility(CursorVisibility::Invisible);
            e.set_echo(false);
            e.set_input_mode(InputMode::Character);
            let tm = match rm {
                RunMode::Stepwise => TimeoutMode::Never,
                RunMode::Normal => TimeoutMode::Immediate,
            };
            e.set_input_timeout(tm);
            EasyCursesRenderer { e, cfg }
        }

        fn refresh(&mut self) {
            self.e.refresh();
        }

        fn handle_input(
            &mut self,
            ch: &mut emulator::Emulator,
            last_input: &mut Instant,
            min_press_durarion: &Duration,
            oldk: &mut Option<usize>,
        ) -> bool {
            let mut result = true;
            if let Some(ip) = self.e.get_input() {
                *last_input = Instant::now();
                match ip {
                    Input::Character(',') => result = false,
                    Input::Character(key) => {
                        if let Some(newk) = key_map::map_key_to_base16(key) {
                            ch.key_pressed(oldk.take(), newk);
                            oldk.replace(newk);
                        }
                    }
                    _ => (),
                }
            } else {
                if let None = min_press_durarion.checked_sub(last_input.elapsed()) {
                    ch.key_released();
                    oldk.take();
                }
            }
            result
        }
    }

    impl<'s> Renderer for EasyCursesRenderer<'s> {
        fn render_cpu(&mut self, cpu: &cpu::CPU) {
            let cfg = self.cfg;
            let e = &mut self.e;
            let (mut r, c) = cfg.cpu_position();

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

            r += 1;
            e.move_rc(r, c);
            render_labelled(
                e,
                "opcode",
                &format!("{:?}", cpu.instr),
                cfg.cpu_width as usize,
            );
        }

        fn render_display(&mut self, d: &dyn display::Scr) {
            let cfg = &self.cfg;
            let e = &mut self.e;
            let (r, c) = cfg.display_position();
            for y in 0i32..display::ROWS as i32 {
                for x in 0i32..display::COLS as i32 {
                    let bit = d.get(x as usize, y as usize);
                    let (z, cp) = if bit {
                        (cfg.present, cfg.color_present)
                    } else {
                        (cfg.absent, cfg.color_absent)
                    };
                    let row = r + y;
                    let col = c + x;
                    e.set_color_pair(cp);
                    e.move_rc(row, col);
                    e.print_char(z);
                }
            }
        }

        fn render_keyboard(&mut self, kbd: &input::Keyboard) {
            let cfg = &self.cfg;
            let e = &mut self.e;
            let (r, c) = cfg.keyboard_position();
            render_kbd_line(e, r, c, [1, 2, 3, 0xC], kbd, cfg);
            render_kbd_line(e, r + 1, c, [4, 5, 6, 0xD], kbd, cfg);
            render_kbd_line(e, r + 2, c, [7, 8, 9, 0xE], kbd, cfg);
            render_kbd_line(e, r + 3, c, [0xA, 0, 0xB, 0xF], kbd, cfg);
        }

        fn render_dt_st(&mut self, dt_st: (u8, u8)) {
            let cfg = &self.cfg;
            let e = &mut self.e;
            let (r, c) = cfg.dt_st_position();
            e.move_rc(r, c);

            let (dt, st) = dt_st;
            e.print(format!("Delay: {:5}, Sound time: {:5}", dt, st));
        }

        fn render_status(&mut self, s: &str) {
            let cfg = &self.cfg;
            let e = &mut self.e;
            let (r, c) = cfg.status_position();
            e.move_rc(r, c);
            let label = "Status";
            part_str(e, label, format!("{:1$}", s, cfg.cpu_width as usize));
        }

        fn render_frame(&mut self) {
            let cfg = &self.cfg;
            let e = &mut self.e;
            let (r, c) = cfg.frame_position();
            let wi = cfg.display_width;
            let hi = cfg.display_height;

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

        fn render_step(&mut self, step: u64, fps: u64) {
            let cfg = &self.cfg;
            let e = &mut self.e;
            let (r, c) = cfg.step_position();
            e.move_rc(r, c);
            let s = format!("Frame {}, fps: {}", step, fps);
            e.print(s);
        }

        fn wait_to_quit(&mut self, s: &str) {
            self.render_status(s);
            self.e.set_input_timeout(TimeoutMode::Never);
            self.e.get_input();
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
        part_str(e, label, format!("{:1$}", s, max_width - label.len()));
    }

    fn part_str(e: &mut EasyCurses, label: &str, s: String) {
        render_line(e, colorpair!(Green on Black), format!("{0:3} ", label));
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

            let key = format!("[{:X}]{} ", num, key_map::map_base16_to_key(num).unwrap());
            e.print(key);
        }
    }

    pub(crate) fn chip_loop<'s>(ch: &mut emulator::Emulator, c: &'s Config, rm: RunMode) {
        let mut step_count = 0u64;
        let mut oldk: Option<usize> = None;
        let mut next_instr = ch.fetch();
        let start_of_prog = Instant::now();
        let mut last_input = Instant::now();

        let frame_target_duration = Duration::new(1, 0)
            .checked_div(120)
            .expect("duration division failed");
        let min_press_durarion = Duration::new(1, 0)
            .checked_div(20)
            .expect("min_press_durarion failed");

        if let Some(mut e) = EasyCurses::initialize_system() {
            let mut er = EasyCursesRenderer::new(&mut e, c, rm);
            er.render_frame();
            er.render_status("Press ',' (colon) to stop emulation.");
            loop {
                let top_of_loop = Instant::now();
                er.render_cpu(&(*ch).cpu);
                er.render_display((*ch).scr.as_ref());
                er.render_keyboard(&ch.kbd);
                if let Some(fps) = step_count.checked_div(start_of_prog.elapsed().as_secs()) {
                    er.render_step(step_count, fps);
                    step_count += 1;
                }
                er.render_dt_st(ch.tick());
                er.refresh();

                if !er.handle_input(ch, &mut last_input, &min_press_durarion, &mut oldk) {
                    break;
                }

                if let Some(instr) = next_instr {
                    ch.exec(instr);
                    next_instr = ch.fetch();
                } else {
                    er.render_status("No more instructions to excute");
                    break;
                }

                let elapsed_this_frame = top_of_loop.elapsed();
                if let Some(frame_remaining) = frame_target_duration.checked_sub(elapsed_this_frame)
                {
                    sleep(frame_remaining);
                }
            }

            er.wait_to_quit("Press any key to quit");
        } else {
            println!("Could not initialize easycurses system properly");
        }
    }

    pub fn default_config() -> Config {
        Config {
            present: '█',
            absent: ' ',
            color_present: ColorPair::new(Yellow, Black),
            color_absent: ColorPair::new(Blue, Black),
            display_width: display::COLS as i32,
            display_height: display::ROWS as i32,
            x0: 3,
            y0: 3,
            cpu_width: 40,
            cpu_height: 26,
            kbd_height: 5,
        }
    }
}

mod mode {
    #[derive(Debug, PartialEq)]
    pub(crate) enum RunMode {
        Stepwise,
        Normal,
    }

    impl std::convert::From<Option<&String>> for RunMode {
        fn from(s: Option<&String>) -> Self {
            match s {
                None => RunMode::Normal,
                Some(_) => RunMode::Stepwise,
            }
        }
    }
}
mod run {

    use crate::mode::RunMode;
    use crate::render;
    use libchip8::emulator;
    use libchip8::loader;

    pub(crate) fn emulation(ch: &mut emulator::Emulator, fname: &str, runmode: RunMode) {
        loader::load(ch, &String::from(fname));
        ch.store_font();

        render::chip_loop(ch, &render::default_config(), runmode);
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn from_none_test() {
            assert_eq!(RunMode::Normal, RunMode::from(None));
        }

        #[test]
        fn from_some_test() {
            assert_eq!(RunMode::Stepwise, RunMode::from(Some(&String::from("-s"))));
        }
    }
}

use libchip8::emulator;
use mode::RunMode;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Some(fname) = &args.get(1) {
        let mut emulator = emulator::Emulator::new();
        let runmode: RunMode = RunMode::from(args.get(2));
        run::emulation(&mut emulator, fname, runmode);
    } else {
        println!("chip-8 rom file name required");
    }
}

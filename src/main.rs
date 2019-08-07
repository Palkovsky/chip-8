use std::default::Default;
use std::collections::HashMap;
use piston_window::*;

/*
 * TYPE ALIASES & CONSTS
 */
type Mem = [u8; RAM_SIZE];
type Keyboard = [bool; KEYBOARD_SIZE];
type Opcode = u16;

const GP_REG_CNT: usize = 16;
const KEYBOARD_SIZE: usize = 16;
const RAM_SIZE: usize = 0x1000;

const DISPLAY_MODE_WIDTH: usize = 128;
const DISPLAY_MODE_HEIGHT: usize = 64;
const DISPLAY_SCALED_WIDTH: usize = DISPLAY_MODE_WIDTH * 5;
const DISPLAY_SCALED_HEIGHT: usize = DISPLAY_MODE_HEIGHT * 5;

/*
 * REGISTERS
 */
#[derive(Debug, Default)]
pub struct Reg {
    /*
     * General purpose regs.
     */
    pub V: [u8; GP_REG_CNT],
    /*
     * Flag register.
     */
    pub VF: bool,
    /*
     *  The delay timer is active whenever the delay timer register (DT) is non-zero.
     *  This timer does nothing more than subtract 1 from the value of DT at a rate of 60Hz. 
     *  When DT reaches 0, it deactivates.
     */
    pub DT: u8,
    /*
     * The sound timer is active whenever the sound timer register (ST) is non-zero.
     * This timer also decrements at a rate of 60Hz, however, as long as ST's value is greater than zero, the Chip-8 buzzer will sound.
     * When ST reaches zero, the sound timer deactivates.
     */
    pub ST: u8,
    /*
     * Program Counter
     */
    pub PC: u16,
    /*
     * Stack Pointer
     */
    pub SP: u8,
}
impl Reg {
    pub fn new() -> Self { Reg { ..Default::default() } }
}

/*
 * DISPLAY
 * Contains sprite drawing logic and image scaling.
 */
pub struct Display {
    r_width: usize,
    r_height: usize,
    s_width: usize,
    s_height: usize,
    h_strech: f64,
    v_strech: f64,
    buffer: Vec<Vec<bool>>,
}

impl Display {
    pub fn new(r_width: usize, s_width: usize, r_height: usize, s_height: usize) -> Self {
        let h_strech = (s_width as f64)/(r_width as f64);
        let v_strech = (s_height as f64)/(r_height as f64);
        Display { 
            r_width: r_width, r_height: r_height,
            s_width: s_width, s_height: s_height,
            h_strech: h_strech, v_strech: v_strech, 
            buffer: vec![vec![false; s_width]; s_height]}
    }

    pub fn cls(&mut self) { self.buffer = vec![vec![false; self.s_width]; self.s_height]; }

    pub fn pixel(&mut self, row: usize, col: usize, update: bool) -> bool {
        if row >= self.r_height { panic!("Tried accessing row {}. Only {} rows available.", row, self.r_height); }
        if col >= self.r_width { panic!("Tried accessing column {}. Only {} columns available.", col, self.r_width); }

        let (x, y) = ((col as f64 * self.h_strech) as usize, (row as f64 * self.v_strech) as usize);
        let (width_scaled, height_scaled) = (self.h_strech as usize, self.v_strech as usize);
        let mut overlap = false;

        for i in y..y+height_scaled {
            for j in x..x+width_scaled {
               if self.buffer[i][j] != update { overlap = true; }
               self.buffer[i][j] = update;
            }
        }

        overlap
    }
}

/*
 * STATE
 * Contains all vartiables needed for executions(regs, memory, dispaly, etc.)
 */
pub struct State {
    pub mem: Mem,
    pub reg: Reg,
    pub display: Display,
    pub key: Keyboard,
}

/*
 * INSTRUCTIONS
 */
pub struct Inst<'a> {
    instructions: HashMap<&'static str, Box<FnMut(Opcode, State)>>,
    state: &'a mut State,
}

impl<'a> Inst<'a>{
    pub fn new(state: &'a mut State) -> Self {
        let instset: Vec<(&'static str, Box<FnMut(Opcode, State)>)> = vec![
            /*
            * 0nnn - SYS addr
            * Jump to a machine code routine at nnn.
            */
            ("0nnn", Box::new(|op, state| {

            })),
            /*
             * 00E0 - CLS
             * Clear the display.  
             */
            ("00EE", Box::new(|op, state| {
            })),
            /*
             * 1nnn - JP addr
             * Jump to location nnn.
             */
            ("1nnn", Box::new(|op, state| {

            })),
            /*
            * 2nnn - CALL addr
            * Call subroutine at nnn.
            * The interpreter increments the stack pointer, then puts the current PC on the top of the stack. The PC is then set to nnn.
            */
            ("2nnn", Box::new(|op, state| {

            })),
            /*
             * 3xkk - SE Vx, byte
             * Skip next instruction if Vx = kk.
             * The interpreter compares register Vx to kk, and if they are equal, increments the program counter by 2.
            */
            ("3xkk", Box::new(|op, state| {

            })),
            /*
             * 4xkk - SNE Vx, byte
             * Skip next instruction if Vx != kk.
             * The interpreter compares register Vx to kk, and if they are not equal, increments the program counter by 2.
             */
            ("4xkk", Box::new(|op, state| {

            })),
            /*
             * 5xy0 - SE Vx, Vy
             * Skip next instruction if Vx = Vy.
             * The interpreter compares register Vx to register Vy, and if they are equal, increments the program counter by 2.
             */
            ("5xy0", Box::new(|op, state| {

            })),
            /*
             * 6xkk - LD Vx, byte
             * Set Vx = kk.
             * The interpreter puts the value kk into register Vx.
             */
            ("6xkk", Box::new(|op, state| {

            })),
            /*
             * 7xkk - ADD Vx, byte
             * Set Vx = Vx + kk.
             * Adds the value kk to the value of register Vx, then stores the result in Vx. 
             */
            ("7xkk", Box::new(|op, state| {

            })),
            /*
             * 8xy0 - LD Vx, Vy
             * Set Vx = Vy.
             * Stores the value of register Vy in register Vx.
             */
            ("8xy0", Box::new(|op, state| {

            })),
            /*
             * 8xy1 - OR Vx, Vy
             * Set Vx = Vx OR Vy.
             * Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx. A bitwise OR compares the corrseponding bits from two values, and if either bit is 1, then the same bit in the result is also 1. Otherwise, it is 0. 
             */
            ("8xy1", Box::new(|op, state| {

            })),
            /*
             * 8xy2 - AND Vx, Vy
             * Set Vx = Vx AND Vy.
             * Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx. A bitwise AND compares the corrseponding bits from two values, and if both bits are 1, then the same bit in the result is also 1. Otherwise, it is 0. 
             */
            ("8xy2", Box::new(|op, state| {

            })),
            /*
             * 8xy3 - XOR Vx, Vy
             * Set Vx = Vx XOR Vy.
             * Performs a bitwise exclusive OR on the values of Vx and Vy, then stores the result in Vx. An exclusive OR compares the corrseponding bits from two values, and if the bits are not both the same, then the corresponding bit in the result is set to 1. Otherwise, it is 0. 
             */
            ("8xy3", Box::new(|op, state| {

            })),
            /*
             * 8xy4 - ADD Vx, Vy
             * Set Vx = Vx + Vy, set VF = carry.
             * The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e., > 255,) VF is set to 1, otherwise 0. Only the lowest 8 bits of the result are kept, and stored in Vx.
             */
            ("8xy4", Box::new(|op, state| {

            })),
            /*
             * 8xy5 - SUB Vx, Vy
             * Set Vx = Vx - Vy, set VF = NOT borrow.
             * If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results stored in Vx.
             */
            ("8xy5", Box::new(|op, state| {

            })),
            /*
             * 8xy6 - SHR Vx {, Vy}
             * Set Vx = Vx SHR 1.
             * If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0. Then Vx is divided by 2.
             */
            ("8xy6", Box::new(|op, state| {

            })),
            /*
             * 8xy7 - SUBN Vx, Vy
             * Set Vx = Vy - Vx, set VF = NOT borrow.
             * If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and the results stored in Vx.
             */
            ("8xy7", Box::new(|op, state| {

            })),
            /*
             * 8xyE - SHL Vx {, Vy}
             * Set Vx = Vx SHL 1.
             * If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to 0. Then Vx is multiplied by 2.
            */
            ("8xyE", Box::new(|op, state| {

            })),
            /*
             * 9xy0 - SNE Vx, Vy
             * Skip next instruction if Vx != Vy.
             * The values of Vx and Vy are compared, and if they are not equal, the program counter is increased by 2.
             */
            ("9xy0", Box::new(|op, state| {

            })),
            /*
             * Annn - LD I, addr
             * Set I = nnn.
             * The value of register I is set to nnn.
             */
            ("Annn", Box::new(|op, state| {

            })),
            /*
             * Bnnn - JP V0, addr
             * Jump to location nnn + V0.
             * The program counter is set to nnn plus the value of V0.
             */
            ("Bnnn", Box::new(|op, state| {

            })),
            /*
             * Cxkk - RND Vx, byte
             * Set Vx = random byte AND kk.
             * The interpreter generates a random number from 0 to 255, which is then ANDed with the value kk. The results are stored in Vx. See instruction 8xy2 for more information on AND.
             */
            ("Cxkk", Box::new(|op, state| {

            })),
            /*
             * Dxyn - DRW Vx, Vy, nibble
             * Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
             * The interpreter reads n bytes from memory, starting at the address stored in I. These bytes are then displayed as sprites on screen at coordinates (Vx, Vy). Sprites are XORed onto the existing screen. If this causes any pixels to be erased, VF is set to 1, otherwise it is set to 0. If the sprite is positioned so part of it is outside the coordinates of the display, it wraps around to the opposite side of the screen. See instruction 8xy3 for more information on XOR, and section 2.4, Display, for more information on the Chip-8 screen and sprites.
             */
            ("Dxyn", Box::new(|op, state| {

            })),
            /*
             * Ex9E - SKP Vx
             * Skip next instruction if key with the value of Vx is pressed.
             * Checks the keyboard, and if the key corresponding to the value of Vx is currently in the down position, PC is increased by 2.
             */
            ("Ex9E", Box::new(|op, state| {

            })),
            /*
             * ExA1 - SKNP Vx
             * Skip next instruction if key with the value of Vx is not pressed.
             * Checks the keyboard, and if the key corresponding to the value of Vx is currently in the up position, PC is increased by 2.
             */
            ("ExA1", Box::new(|op, state| {

            })),
            /*
             * Fx07 - LD Vx, DT
             * Set Vx = delay timer value.
             * The value of DT is placed into Vx.
             */
            ("Fx07", Box::new(|op, state| {

            })),
            /*
             * Fx0A - LD Vx, K
             * Wait for a key press, store the value of the key in Vx.
             * All execution stops until a key is pressed, then the value of that key is stored in Vx.
             */
            ("Fx0A", Box::new(|op, state| {

            })),
            /*
             * Fx15 - LD DT, Vx
             * Set delay timer = Vx.
             * DT is set equal to the value of Vx.
             */
            ("Fx15", Box::new(|op, state| {

            })),
            /*
             * Fx18 - LD ST, Vx
             * Set sound timer = Vx.
             * ST is set equal to the value of Vx.
             */
            ("Fx18", Box::new(|op, state| {

            })),
            /*
             * Fx1E - ADD I, Vx
             * Set I = I + Vx.
             * The values of I and Vx are added, and the results are stored in I.
             */
            ("Fx1E", Box::new(|op, state| {

            })),
            /*
             * Fx29 - LD F, Vx
             * Set I = location of sprite for digit Vx.
             * The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx. See section 2.4, Display, for more information on the Chip-8 hexadecimal font.
             */
            ("Fx29", Box::new(|op, state| {

            })),
            /*
             * Fx33 - LD B, Vx
             * Store BCD representation of Vx in memory locations I, I+1, and I+2.
             * The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2.
             */
            ("Fx33", Box::new(|op, state| {

            })),
            /*
             * Fx55 - LD [I], Vx
             * Store registers V0 through Vx in memory starting at location I.
             * The interpreter copies the values of registers V0 through Vx into memory, starting at the address in I.
             */
            ("Fx55", Box::new(|op, state| {

            })),
            /*
             * Fx65 - LD Vx, [I]
             * Read registers V0 through Vx from memory starting at location I.
             * The interpreter reads values from memory starting at location I into registers V0 through Vx.
             */
            ("Fx65", Box::new(|op, state| {

            })),
        ];
        
        Inst { instructions: instset.into_iter().collect(), state: state }
    }

    pub fn exec(&mut self, op: Opcode) {
        //self.instructions.get(op)
    }
}

fn map_keyboard(keyboard: &mut Keyboard, e: &Event) {
    let translation: HashMap<Key, usize> = vec![
        Key::D1, Key::D2, Key::D3, Key::D4,
        Key::Q, Key::W, Key::E, Key::R,
        Key::A, Key::S, Key::D, Key::F,
        Key::Z, Key::X, Key::C, Key::V,
    ].into_iter().enumerate().map(|(i, key)| (key, i)).collect();

    if let Some(Button::Keyboard(key)) = e.press_args() {
        if let Some(idx) = translation.get(&key) { keyboard[*idx] = true; }
    }

    if let Some(Button::Keyboard(key)) = e.release_args() {
        if let Some(idx) = translation.get(&key) { keyboard[*idx] = false; }
    }
}

fn main() {
    let mem = [0u8; RAM_SIZE];
    let reg = Reg::new();
    let display = Display::new(DISPLAY_MODE_WIDTH, DISPLAY_SCALED_WIDTH, DISPLAY_MODE_HEIGHT, DISPLAY_SCALED_HEIGHT);
    let key = [false; KEYBOARD_SIZE];

    let mut state = State {mem: mem, reg: reg, display: display, key: key};
    let mut inst = Inst::new(&mut state);

    let dimen = Size {width: state.display.s_width as f64, height: state.display.s_height as f64};
    let mut window: PistonWindow = WindowSettings::new("Chip-8 Emu", dimen)
        .exit_on_esc(true)
        .resizable(false)
        .build().unwrap();

    let mut events = Events::new(EventSettings::new().lazy(true));
    while let Some(e) = events.next(&mut window) { 
        map_keyboard(&mut state.key, &e);

        state.display.pixel(state.display.r_height-1, state.display.r_width-1, state.key[0]);
        state.display.pixel(state.display.r_height-1, 0, state.key[0]);
        state.display.pixel(0, 0, state.key[0]);
        state.display.pixel(0, state.display.r_width-1, state.key[0]);

        window.draw_2d(&e, |context, graphics, _| {
            clear([0.0; 4], graphics);
            for i in 0..state.display.s_height {
                for j in 0..state.display.s_width {
                    if state.display.buffer[i][j] {
                        rectangle([1.0; 4], [j as f64, i as f64, 1.0, 1.0], context.transform, graphics);
                    }
                }
            }
        });
    }
}

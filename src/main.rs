use std::default::Default;
use std::collections::HashMap;
use std::num::Wrapping;
use std::{fs, env, path};
use piston_window::*;
use rodio::{Sink, Source};
use rand::Rng;

/*
 * TYPE ALIASES & CONSTS
 */
type Mem = [u8; RAM_SIZE];
type Stack = [u16; STACK_SIZE];
type Keyboard = [bool; KEYBOARD_SIZE];
type Op = u16;
type ChunkedOp = (usize, usize, usize, usize);

const GP_REG_CNT: usize = 16;
const KEYBOARD_SIZE: usize = 16;
const RAM_SIZE: usize = 0x1000;
const STACK_SIZE: usize = 16;

const DISPLAY_MODE_WIDTH: usize = 64;
const DISPLAY_MODE_HEIGHT: usize = 32;
const DISPLAY_SCALED_WIDTH: usize = DISPLAY_MODE_WIDTH * 10;
const DISPLAY_SCALED_HEIGHT: usize = DISPLAY_MODE_HEIGHT * 10;
const ENTRY_POINT: u16 = 0x200;

/*
 * REGISTERS
 */
#[derive(Debug, Default)]
pub struct Reg {
    /*
     * General purpose regs.
     */
    pub V: [u8; GP_REG_CNT],
    pub I: u16,
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
    pub fn new() -> Self { 
        Reg {..Default::default() } 
    }

    pub fn update_DT(&mut self) {
        if self.DT > 0 { self.DT -= 1; }
    }

    pub fn update_ST(&mut self, audio: &Audio) {
        if self.ST > 0 {
            if !audio.is_playing() { audio.play(); }
            self.ST -= 1;
            if self.ST == 0 { audio.stop(); }
        }
    }
}
/*
 * AUDIO
 */
pub struct Audio {
    player: rodio::Sink,
}
impl Audio {
    pub fn new() -> Self {
        let device = rodio::default_output_device().unwrap();
        let sink = Sink::new(&device);

        sink.set_volume(0.75);
        Audio { player: sink }
    }

    pub fn play(&self) {
        if self.player.len() == 0 {
            let source = rodio::source::SineWave::new(300).repeat_infinite();
            self.player.append(source);
        }
        self.player.play();
    }

    pub fn stop(&self) {
        self.player.pause();
    }

    pub fn is_playing(&self) -> bool {
        !self.player.is_paused() && self.player.len() > 0
    }
}
/*
 * DISPLAY
 * Contains sprite drawing logic and image scaling.
 */
pub struct Display {
    pub width: usize,
    pub height: usize,
    buffer: Vec<Vec<bool>>,
}

impl Display {
    pub fn new(width: usize, height: usize) -> Self {
        Display { 
            width: width, height: height,
            buffer: vec![vec![false; width]; height]
        }
    }

    pub fn cls(&mut self) { self.buffer = vec![vec![false; self.width]; self.height]; }

    pub fn pixel(&mut self, row: usize, col: usize, update: bool) -> bool {
        let row = row % self.height;
        let col = col % self.width;
        
        let updated = self.buffer[row][col] ^ update;
        let overriden = self.buffer[row][col] != update;

        self.buffer[row][col] = updated;
        overriden
    }
}
/*
 * STATE
 * Contains all vartiables needed for executions(regs, memory, dispaly, etc.)
 */
pub struct State {
    pub mem: Mem,
    pub stack: Stack,
    pub reg: Reg,
    pub display: Display,
    pub audio: Audio,
    pub key: Keyboard,
}
/*
 * INSTRUCTIONS
 */
pub struct Inst {
    instructions: HashMap<&'static str, Box<FnMut(ChunkedOp, &mut State)>>,
}
impl Inst{
    pub fn new() -> Self {
        let instset: Vec<(&'static str, Box<FnMut(ChunkedOp, &mut State)>)> = vec![
            /*
             * 00E0 - CLS
             * Clear the display.  
             */
            ("00E0", Box::new(|_, state| state.display.cls())),
            /*
            * 00EE - RET
            * Return from a subroutine.
            * The interpreter sets the program counter to the address at the top of the stack, then subtracts 1 from the stack pointer.
            */
            ("00EE", Box::new(|_, state| {
                state.reg.PC = state.stack[state.reg.SP as usize];
                state.reg.SP -= 1;
            })),
            /*
             * 1nnn - JP addr
             * Jump to location nnn.
             */
            ("1nnn", Box::new(|(_, a, b, c), state| {
                let addr = ((a << 8) + (b << 4) + c) as u16;
                state.reg.PC = addr;
            })),
            /*
            * 2nnn - CALL addr
            * Call subroutine at nnn.
            * The interpreter increments the stack pointer, then puts the current PC on the top of the stack. The PC is then set to nnn.
            */
            ("2nnn", Box::new(|(_, a, b, c), state| {
                state.reg.SP += 1;
                state.stack[state.reg.SP as usize] = state.reg.PC;
                let addr = ((a << 8) + (b << 4) + c) as u16;
                state.reg.PC = addr;
            })),
            /*
             * 3xkk - SE Vx, byte
             * Skip next instruction if Vx = kk.
             * The interpreter compares register Vx to kk, and if they are equal, increments the program counter by 2.
            */
            ("3xkk", Box::new(|(_, x, k1, k2), state| {
                let kk = ((k1 << 4) + k2) as u8;
                if state.reg.V[x] == kk {
                    state.reg.PC += 2;
                }
            })),
            /*
             * 4xkk - SNE Vx, byte
             * Skip next instruction if Vx != kk.
             * The interpreter compares register Vx to kk, and if they are not equal, increments the program counter by 2.
             */
            ("4xkk", Box::new(|(_, x, k1, k2), state| {
                let kk = ((k1 << 4) + k2) as u8;
                if state.reg.V[x] != kk {
                    state.reg.PC += 2;
                }
            })),
            /*
             * 5xy0 - SE Vx, Vy
             * Skip next instruction if Vx = Vy.
             * The interpreter compares register Vx to register Vy, and if they are equal, increments the program counter by 2.
             */
            ("5xy0", Box::new(|(_, x, y, _), state| {
                if state.reg.V[x] == state.reg.V[y] {
                    state.reg.PC += 2;
                }
            })),
            /*
             * 6xkk - LD Vx, byte
             * Set Vx = kk.
             * The interpreter puts the value kk into register Vx.
             */
            ("6xkk", Box::new(|(_, x, k1, k2), state| {
                let kk = ((k1 << 4) + k2) as u8;
                state.reg.V[x] = kk;
            })),
            /*
             * 7xkk - ADD Vx, byte
             * Set Vx = Vx + kk.
             * Adds the value kk to the value of register Vx, then stores the result in Vx. 
             */
            ("7xkk", Box::new(|(_, x, k1, k2), state| {
                let kk = ((k1 << 4) + k2) as u8;
                state.reg.V[x] = (Wrapping(state.reg.V[x]) + Wrapping(kk)).0;
            })),
            /*
             * 8xy0 - LD Vx, Vy
             * Set Vx = Vy.
             * Stores the value of register Vy in register Vx.
             */
            ("8xy0", Box::new(|(_, x, y, _), state| {
                state.reg.V[x] = state.reg.V[y];
            })),
            /*
             * 8xy1 - OR Vx, Vy
             * Set Vx = Vx OR Vy.
             * Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx. A bitwise OR compares the corrseponding bits from two values, and if either bit is 1, then the same bit in the result is also 1. Otherwise, it is 0. 
             */
            ("8xy1", Box::new(|(_, x, y, _), state| {
                state.reg.V[x] |= state.reg.V[y];
            })),
            /*
             * 8xy2 - AND Vx, Vy
             * Set Vx = Vx AND Vy.
             * Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx. A bitwise AND compares the corrseponding bits from two values, and if both bits are 1, then the same bit in the result is also 1. Otherwise, it is 0. 
             */
            ("8xy2", Box::new(|(_, x, y, _), state| {
                state.reg.V[x] &= state.reg.V[y];
            })),
            /*
             * 8xy3 - XOR Vx, Vy
             * Set Vx = Vx XOR Vy.
             * Performs a bitwise exclusive OR on the values of Vx and Vy, then stores the result in Vx. An exclusive OR compares the corrseponding bits from two values, and if the bits are not both the same, then the corresponding bit in the result is set to 1. Otherwise, it is 0. 
             */
            ("8xy3", Box::new(|(_, x, y, _), state| {
                state.reg.V[x] ^= state.reg.V[y];
            })),
            /*
             * 8xy4 - ADD Vx, Vy
             * Set Vx = Vx + Vy, set VF = carry.
             * The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e., > 255,) VF is set to 1, otherwise 0. Only the lowest 8 bits of the result are kept, and stored in Vx.
             */
            ("8xy4", Box::new(|(_, x, y, _), state| {
                state.reg.V[0xF] = if (state.reg.V[x] as u16 + state.reg.V[y] as u16) > 255 {1} else {0};
                state.reg.V[x] = (Wrapping(state.reg.V[x]) + Wrapping(state.reg.V[y])).0;
            })),
            /*
             * 8xy5 - SUB Vx, Vy
             * Set Vx = Vx - Vy, set VF = NOT borrow.
             * If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results stored in Vx.
             */
            ("8xy5", Box::new(|(_, x, y, _), state| {
                state.reg.V[0xF] = if state.reg.V[x] > state.reg.V[y] {1} else {0};
                state.reg.V[x] = (Wrapping(state.reg.V[x]) - Wrapping(state.reg.V[y])).0;
            })),
            /*
             * 8xy6 - SHR Vx {, Vy}
             * Set Vx = Vx SHR 1.
             * If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0. Then Vx is divided by 2.
             */
            ("8xy6", Box::new(|(_, x, _, _), state| {
                state.reg.V[0xF] = if state.reg.V[x] & 0x01 != 0 {1} else {0};
                state.reg.V[x] = state.reg.V[x] >> 1;
            })),
            /*
             * 8xy7 - SUBN Vx, Vy
             * Set Vx = Vy - Vx, set VF = NOT borrow.
             * If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and the results stored in Vx.
             */
            ("8xy7", Box::new(|(_, x, y, _), state| {
                state.reg.V[0xF] = if state.reg.V[y] > state.reg.V[x] {1} else {0};
                state.reg.V[x] = (Wrapping(state.reg.V[y]) - Wrapping(state.reg.V[x])).0;
            })),
            /*
             * 8xyE - SHL Vx {, Vy}
             * Set Vx = Vx SHL 1.
             * If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to 0. Then Vx is multiplied by 2.
            */
            ("8xyE", Box::new(|(_, x, _, _), state| {
                state.reg.V[0xF] = if state.reg.V[x] & 0x80 != 0 {1} else {0};
                state.reg.V[x] = state.reg.V[x] << 1;
            })),
            /*
             * 9xy0 - SNE Vx, Vy
             * Skip next instruction if Vx != Vy.
             * The values of Vx and Vy are compared, and if they are not equal, the program counter is increased by 2.
             */
            ("9xy0", Box::new(|(_, x, y, _), state| {
                if state.reg.V[x] != state.reg.V[y] {
                    state.reg.PC += 2;
                } 
            })),
            /*
             * Annn - LD I, addr
             * Set I = nnn.
             * The value of register I is set to nnn.
             */
            ("Annn", Box::new(|(_, a, b, c), state| {
                let val = ((a as u16) << 8) + ((b as u16) << 4) + c as u16;
                state.reg.I = val;
            })),
            /*
             * Bnnn - JP V0, addr
             * Jump to location nnn + V0.
             * The program counter is set to nnn plus the value of V0.
             */
            ("Bnnn", Box::new(|(_, a, b, c), state| {
                let addr = ((a << 8) + (b << 4) + c) as u16;
                state.reg.PC = addr + (state.reg.V[0] as u16);
            })),
            /*
             * Cxkk - RND Vx, byte
             * Set Vx = random byte AND kk.
             * The interpreter generates a random number from 0 to 255, which is then ANDed with the value kk. The results are stored in Vx. See instruction 8xy2 for more information on AND.
             */
            ("Cxkk", Box::new(|(_, x, k1, k2), state| {
                let kk = ((k1 << 4) + k2) as u8;
                let rnd = rand::thread_rng().gen::<u8>();
                state.reg.V[x] = rnd & kk;
            })),
            /*
             * Dxyn - DRW Vx, Vy, nibble
             * Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
             * The interpreter reads n bytes from memory, starting at the address stored in I. These bytes are then displayed as sprites on screen at coordinates (Vx, Vy).
             * Sprites are XORed onto the existing screen. If this causes any pixels to be erased, VF is set to 1, otherwise it is set to 0. If the sprite is positioned so part of it is outside the coordinates of the display, it wraps around to the opposite side of the screen.
             */
            ("Dxyn", Box::new(|(_, x, y, n), state| {
                let addr = state.reg.I as usize;
                let bytes = &state.mem[addr..addr+n];
                
                state.reg.V[0xF] = 0;

                let mut row = state.reg.V[y] as usize;
                for byte in bytes {  
                    let mut mask = 0x80; 
                    let mut col = state.reg.V[x] as usize;
                    while mask != 0 {
                        if byte & mask != 0 && state.display.buffer[row][col] { state.reg.V[0xF] = 1; }
                        state.display.pixel(row, col, byte & mask != 0);
                        mask = mask >> 1;
                        col += 1;

                    }
                    row += 1;
                }
            })),
            /*
             * Ex9E - SKP Vx
             * Skip next instruction if key with the value of Vx is pressed.
             * Checks the keyboard, and if the key corresponding to the value of Vx is currently in the down position, PC is increased by 2.
             */
            ("Ex9E", Box::new(|(_, x, _, _), state| {
                if state.key[state.reg.V[x] as usize] {
                    state.reg.PC += 2;
                }
            })),
            /*
             * ExA1 - SKNP Vx
             * Skip next instruction if key with the value of Vx is not pressed.
             * Checks the keyboard, and if the key corresponding to the value of Vx is currently in the up position, PC is increased by 2.
             */
            ("ExA1", Box::new(|(_, x, _, _), state| {
                if !state.key[state.reg.V[x] as usize] {
                    state.reg.PC += 2;
                }
            })),
            /*
             * Fx07 - LD Vx, DT
             * Set Vx = delay timer value.
             * The value of DT is placed into Vx.
             */
            ("Fx07", Box::new(|(_, x, _, _), state| {
                state.reg.V[x] = state.reg.DT;
            })),
            /*
             * Fx0A - LD Vx, K
             * Wait for a key press, store the value of the key in Vx.
             * All execution stops until a key is pressed, then the value of that key is stored in Vx.
             */
            ("Fx0A", Box::new(|(_, x, _, _), state| {
                //panic!("Fx0A unsuporrted");
                state.reg.V[x] = 1;
            })),
            /*
             * Fx15 - LD DT, Vx
             * Set delay timer = Vx.
             * DT is set equal to the value of Vx.
             */
            ("Fx15", Box::new(|(_, x, _, _), state| {
                state.reg.DT = state.reg.V[x];
            })),
            /*
             * Fx18 - LD ST, Vx
             * Set sound timer = Vx.
             * ST is set equal to the value of Vx.
             */
            ("Fx18", Box::new(|(_, x, _, _), state| {
                state.reg.ST = state.reg.V[x];
            })),
            /*
             * Fx1E - ADD I, Vx
             * Set I = I + Vx.
             * The values of I and Vx are added, and the results are stored in I.
             */
            ("Fx1E", Box::new(|(_, x, _, _), state| {
                state.reg.I = (Wrapping(state.reg.I) + Wrapping(state.reg.V[x] as u16)).0;
            })),
            /*
             * Fx29 - LD F, Vx
             * Set I = location of sprite for digit Vx.
             * The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx. See section 2.4, Display, for more information on the Chip-8 hexadecimal font.
             */
            ("Fx29", Box::new(|(_, x, _, _), state| {
                let digit = state.reg.V[x] as u16;
                state.reg.I = digit * 5;
            })),
            /*
             * Fx33 - LD B, Vx
             * Store BCD representation of Vx in memory locations I, I+1, and I+2.
             * The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2.
             */
            ("Fx33", Box::new(|(_, x, _, _), state| {
                let mut num = state.reg.V[x];
                for i in 2..0 {
                    state.mem[state.reg.I as usize + i] = num % 10;
                    num /= 10;
                }
            })),
            /*
             * Fx55 - LD [I], Vx
             * Store registers V0 through Vx in memory starting at location I.
             * The interpreter copies the values of registers V0 through Vx into memory, starting at the address in I.
             */
            ("Fx55", Box::new(|(_, x, _, _), state| {
                let start = state.reg.I as usize;
                for i in 0..x+1 {
                    state.mem[start + i] = state.reg.V[i];
                }
            })),
            /*
             * Fx65 - LD Vx, [I]
             * Read registers V0 through Vx from memory starting at location I.
             * The interpreter reads values from memory starting at location I into registers V0 through Vx.
             */
            ("Fx65", Box::new(|(_, x, _, _), state| {
                let start = state.reg.I as usize;
                for i in 0..x+1 {
                    state.reg.V[i] = state.mem[start + i];
                }
            })),
        ];
        
        Inst { instructions: instset.into_iter().collect() }
    }

    /*
     * Fetches, decodes and executes opcode.
     */
    pub fn exec(&mut self, state: &mut State) {
        // Fetch
        let (upper, lower) = (state.mem[state.reg.PC as usize] as u16, state.mem[state.reg.PC as usize+1] as u16);
        let op = ((upper << 8) + lower) as usize;
        state.reg.PC += 2;

        // Decode
        let bits = ((op >> 12) & 0xF, (op >> 8) & 0xF, (op >> 4) & 0xF, op & 0xF);
        let key = match bits {
            (0x0, 0x0, 0xE, 0x0) => "00E0",
            (0x0, 0x0, 0xE, 0xE) => "00EE",
            (0x1, _, _, _)       => "1nnn",
            (0x2, _, _, _)       => "2nnn",
            (0x3, _, _, _)       => "3xkk",
            (0x4, _, _, _)       => "4xkk",
            (0x5, _, _, 0x0)     => "5xy0",
            (0x6, _, _, _)       => "6xkk",
            (0x7, _, _, _)       => "7xkk",
            (0x8, _, _, 0x0)     => "8xy0",
            (0x8, _, _, 0x1)     => "8xy1",
            (0x8, _, _, 0x2)     => "8xy2",
            (0x8, _, _, 0x3)     => "8xy3",
            (0x8, _, _, 0x4)     => "8xy4",
            (0x8, _, _, 0x5)     => "8xy5",
            (0x8, _, _, 0x6)     => "8xy6",
            (0x8, _, _, 0x7)     => "8xy7",
            (0x8, _, _, 0xE)     => "8xyE",
            (0x9, _, _, 0x0)     => "9xy0",
            (0xA, _, _, _)       => "Annn",
            (0xB, _, _, _)       => "Bnnn",
            (0xC, _, _, _)       => "Cxkk",
            (0xD, _, _, _)       => "Dxyn",
            (0xE, _, 0x9, 0xE)   => "Ex9E",
            (0xE, _, 0xA, 0x1)   => "ExA1",
            (0xF, _, 0x0, 0x7)   => "Fx07",
            (0xF, _, 0x0, 0xA)   => "Fx0A",
            (0xF, _, 0x1, 0x5)   => "Fx15",
            (0xF, _, 0x1, 0x8)   => "Fx18",
            (0xF, _, 0x1, 0xE)   => "Fx1E",
            (0xF, _, 0x2, 0x9)   => "Fx29",
            (0xF, _, 0x3, 0x3)   => "Fx33",
            (0xF, _, 0x5, 0x5)   => "Fx55",
            (0xF, _, 0x6, 0x5)   => "Fx65",
            _ => panic!("Invalid insturction: {:?} | Hex: {:X}", bits, op),
        };

        // Execute
        let func = self.instructions.get_mut(key)
            .unwrap_or_else(|| panic!("Invalid insturction: {:?} | Hex: {:X}", bits, op));
        func(bits, state);
    }
}

fn map_keyboard(keyboard: &mut Keyboard, inp: &Input) {
    let translation: HashMap<Key, usize> = vec![
        Key::D1, Key::D2, Key::D3, Key::D4,
        Key::Q, Key::W, Key::E, Key::R,
        Key::A, Key::S, Key::D, Key::F,
        Key::Z, Key::X, Key::C, Key::V,
    ].into_iter().enumerate().map(|(i, key)| (key, i)).collect();

    if let Input::Button(but) = inp {
        if let Button::Keyboard(key) = but.button {
            let pressed = but.state == ButtonState::Press;
            if let Some(idx) = translation.get(&key) { keyboard[*idx] = pressed; }
        }
    }
}

fn main() {
    if env::args().len() != 2 { panic!("Usage: {} [path]", env::args().nth(0).unwrap()); }

    // Open File -> Read File -> Convert to vector of Opcodes
    let filename = env::args().nth(1).unwrap();
    let bytes = fs::read(path::Path::new(&filename))
        .unwrap_or_else(|_| panic!("Unable to read {}", filename));

    // Assemble all VM components
    let mem = [0u8; RAM_SIZE];
    let stack = [0u16; STACK_SIZE];
    let reg = Reg::new();
    let display = Display::new(DISPLAY_MODE_WIDTH, DISPLAY_MODE_HEIGHT);
    let audio = Audio::new();
    let key = [false; KEYBOARD_SIZE];

    // And put them into State struct
    let mut state = State {mem: mem, stack: stack, reg: reg, display: display, audio: audio, key: key};

    // Load bytes to memory
    for (i, b) in bytes.into_iter().enumerate() { state.mem[ENTRY_POINT as usize + i] = b; }
    state.reg.PC = ENTRY_POINT;

    // Preload hex digits
    let digits = vec![
        0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
        0x20, 0x60, 0x20, 0x20, 0x70, // 1
        0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
        0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
        0x90, 0x90, 0xF0, 0x10, 0x10, // 4
        0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
        0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
        0xF0, 0x10, 0x20, 0x40, 0x40, // 7
        0xF0, 0x90, 0xF0, 0x90, 0x90, // 8
        0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
        0xF0, 0x90, 0xF0, 0x90, 0x90, // A
        0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
        0xF0, 0x80, 0x80, 0x80, 0xF0, // C
        0xE0, 0x90, 0x90, 0x90, 0xE0, // D
        0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
        0xF0, 0x80, 0xF0, 0x80, 0x80, // F
    ];
    for (i, b) in digits.iter().enumerate() { state.mem[i] = *b; }

    // Inst struct let's you execute instructions.
    let mut inst = Inst::new();

    // Initialize Piston window
    let dimen = Size {width: DISPLAY_SCALED_WIDTH as f64, height: DISPLAY_SCALED_HEIGHT as f64};
    let mut window: PistonWindow = WindowSettings::new("Chip-8 Emu", dimen)
        .exit_on_esc(true).resizable(false)
        .build().unwrap();

    while let Some(e) = window.next() {
        match e {
            /*
             * INPUT
             */
            Event::Input(inp, _) =>  { map_keyboard(&mut state.key, &inp); },
            /*
             * UPDATE
             */
            Event::Loop(Loop::Update(_)) => {
                for _ in 0..5 {
                    state.reg.update_ST(&state.audio);
                    state.reg.update_DT();
                    inst.exec(&mut state);
                }
            },
            /*
             * RENDER
             */
            Event::Loop(Loop::Render(_)) => {  
                let h_strech = (DISPLAY_SCALED_WIDTH as f64)/(state.display.width as f64);
                let v_strech = (DISPLAY_SCALED_HEIGHT as f64)/(state.display.height as f64);
    
                window.draw_2d(&e, |context, graphics, _| {
                    clear([0.0; 4], graphics);
                    for i in 0..state.display.height {
                        for j in 0..state.display.width {
                            let x = (j as f64) * h_strech;
                            let y = (i as f64) * v_strech;

                            if state.display.buffer[i][j] {
                                rectangle([1.0; 4], [x, y, h_strech, v_strech], context.transform, graphics);
                            }
                        }
                    }
                });
            },
            _ => {}
        }
    }
}

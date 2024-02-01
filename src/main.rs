use std::env;
use std::fs::read_to_string;
use std::process::exit;
use std::mem::transmute;
use vonneumann::ExecutableMemory;

const COMMANDS: [char; 8] = ['+','-','>','<','.',',','[',']'];
const MAX_MEM: usize = 32767;

#[derive(Debug, Clone, Copy)]
enum CMD {
    Plus(u8),
    Minus(u8),
    PtrR(u32),
    PtrL(u32),
    Push(u16),
    Pull,
    JmpR(usize),
    JmpL(usize),
}

fn get_32bit_offset(jump_from: usize, jump_to: usize) -> u32 {
    if jump_to >= jump_from {
        let diff = jump_to - jump_from;
        // println!("1: {}", diff as i64);
        return diff as u32;
    } else {
        let diff = jump_from - jump_to;
        let diff_unsigned = diff as u32;
        // println!("2: {}", diff_unsigned as i64);
        return !diff_unsigned.wrapping_sub(1);
    }
}

struct Buff {
    data: Vec<u8>,
    jmp_stack: Vec<usize>,
}

impl<'a> Buff {
    fn push(&mut self, v: u8) {
        self.data.push(v);
    }
    fn append(&mut self, vec: Vec<u8>) {
        for v in vec {
            self.push(v)
        }
    }
    fn stack(&mut self, v: usize) {
        self.jmp_stack.push(v);
    }
    fn len(&self) -> usize {
        self.data.len()
    }
    fn u32(&mut self, value: u32) {
        let bytes = value.to_le_bytes();
        self.data.extend_from_slice(&bytes);
    }
    fn u64(&mut self, value: u64) {
        let bytes = value.to_le_bytes();
        self.data.extend_from_slice(&bytes);
    }
    fn _replace_u64(&mut self, value: u64, index: usize) {
        let bytes = value.to_le_bytes();
        for (i, byte) in bytes.iter().enumerate() {
            self.data[index + i] = *byte;
        }
    }
    fn replace_u32(&mut self, value: u32, index: usize) {
        let bytes = value.to_le_bytes();
        for (i, &byte) in bytes.iter().enumerate() {
            if let Some(elem) = self.data.get_mut(index + i) {
                *elem = byte;
            }
        }
    }
    fn encode(&mut self, cmds: Vec<CMD>, mem_ptr: *const u8) {
        // movabs r13 (used for program)
        self.append(vec![0x49, 0xbd]);
        self.u64(mem_ptr as u64);
        // movabs r14 (tracks maximum)
        // self.append(vec![0x49, 0xbe]);
        // self.u64(mem_ptr + MAX_MEM as u64);
        // movabs r15 (tracks minimum)
        // self.append(vec![0x49, 0xbf]);
        // self.u64(mem_ptr);

        for cmd in cmds {
            println!(":{:?}", cmd);
            match cmd {
                CMD::Plus(n) => {
// self.append(vec![0x4D, 0x39, 0xF5, 0x7E, 0x07, 0x4D, 0x89, 0xF5, 0xE9, 0x0C, 0x00, 0x00, 0x00]);
// self.append(vec![0x4D, 0x39, 0xFD, 0x7D, 0x07, 0x4D, 0x89, 0xFD, 0xEB, 0x05]);
                    // increment the value r13 points at by n (8bit)
                    self.append(vec![0x41, 0x80, 0x45, 0x00, n]);

                    // 4D 39 F5 7E 02 4D 89 F5
                    // 4D 39 FD 7D 02 4D 89 FD
                },
                CMD::Minus(n) => {
                    // deincrement the value r13 points at by n (8bit)
                    self.append(vec![0x41, 0x80, 0x6D, 0x00, n]);
                },
                CMD::PtrR(n) => {
                    // increment r13 by n (8bit)
                    if n <= 255 {
                        self.append(vec![0x49, 0x83, 0xC5, n as u8]);
                    } else {
                        let bytes = n.to_le_bytes();
                        self.append(vec![0x49, 0x81, 0xC5, bytes[0], bytes[1], bytes[2], bytes[3]]);
                    }
                },
                CMD::PtrL(n) => {
                    // increment r13 by n (8bit)
                    if n <= 255 {
                        self.append(vec![0x49, 0x83, 0xED, n as u8]);
                    } else {
                        let bytes = n.to_le_bytes();
                        self.append(vec![0x49, 0x81, 0xED, bytes[0], bytes[1], bytes[2], bytes[3]]);
                    }
                },
                CMD::Push(n) => {
                    self.append(vec![0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00]);
                    self.append(vec![0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00]);
                    self.append(vec![0x4C, 0x89, 0xEE]);
                    self.append(vec![0x48, 0xC7, 0xC2, 0x01, 0x00, 0x00, 0x00]);
                    // once registers are setup, we can repeat our systemcall if desired
                    for _ in 0..n {
                        self.append(vec![0x0F, 0x05]);
                    }
                },
                CMD::Pull => {
                    self.append(vec![0x48, 0xC7, 0xC0, 0x00, 0x00, 0x00, 0x00]);
                    self.append(vec![0x48, 0xC7, 0xC7, 0x00, 0x00, 0x00, 0x00]);
                    self.append(vec![0x4C, 0x89, 0xEE]);
                    self.append(vec![0x48, 0xC7, 0xC2, 0x01, 0x00, 0x00, 0x00]);
                    self.append(vec![0x0F, 0x05]);
                },
                CMD::JmpR(_) => {
                    self.append(vec![0x41, 0x80, 0x7D, 0x00, 0x00]);
                    // append the current position to stack
                    self.stack(self.len());

                    // append placeholder jump location
                    self.u32(0_u32);
                },
                CMD::JmpL(_) => {
                    // get the location of the most recent JmpR
                    match self.jmp_stack.pop() {
                        None => {
                            eprintln!("Mismatched brackets @: {}", self.len());
                            break
                        },
                        Some(open_offset) => {
                            self.append(vec![0x41, 0x80, 0x7D, 0x00, 0x00]);
                            // calc value to jump to
                            let jmp_bk_from = self.len() + 6;
                            let jmp_bk_to = open_offset + 6;
                            let rel_jmp_bk_offset = get_32bit_offset(jmp_bk_from, jmp_bk_to);
                            // append it
                            self.append(vec![0x0F, 0x85]);
                            self.u32(rel_jmp_bk_offset);
                            // calculate value jump back from
                            let jmp_fw_from = open_offset + 6;
                            let jmp_fw_to = self.len();
                            let rel_jmp_fw_offset = get_32bit_offset(jmp_fw_from, jmp_fw_to);
                            // overwrite the placeholder value with it
                            self.replace_u32(rel_jmp_fw_offset, open_offset + 2);
                        },
                    }
                },
            }
            show_hex_32(&self.data);
            println!("-");
        }
        // append return value
        // for calling convertion
        self.push(0xc3);
    }
}

// attempts to read the first arg as file to string
// will Panic! if the file doesn't exist or cannot be read
fn read_input_file() -> String {
    let args: Vec<String> = env::args().collect(); // get command line arguments

    if args.len() != 2 {
        println!("Incorrect number of args.");
        eprintln!("Usage: {} <filename>", args[0]);
        exit(1)
    }

    let filepath = &args[1]; // the last argument is the file name

    match read_to_string(filepath) {
        Ok(content) => content,
        Err(e) => panic!("Failed to read file, err: {}", e),
    }
}

fn parse(code: &mut Vec<char>) -> Vec<CMD> {
    let mut map: Vec<CMD> = vec![];
    code.push(' ');
    let mut jmp_stack: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < code.len() {
        match code[i] {
            '+' => {
                let mut l = 0;
                while code[i] == '+' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Plus(l));
            },
            '-' => {
                let mut l = 0;
                while code[i] == '-' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Minus(l));
            },
            '>' => {
                let mut l = 0;
                while code[i] == '>' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::PtrR(l));
            },
            '<' => {
                let mut l = 0;
                while code[i] == '<' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::PtrL(l));
            },
            '.' => {
                let mut l = 0;
                while code[i] == '.' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Push(l));
            },
            ',' => {
                map.push(CMD::Pull);
                i += 1;
            },
            '[' => {
                map.push(CMD::JmpR(0));
                jmp_stack.push(i);
                i += 1;
            },
            ']' => {
                let offset = jmp_stack.pop();
                match offset {
                    Some(num) => {
                        let diff = i - num;
                        map.push(CMD::JmpL(diff));
                    },
                    None => {},
                }
                i += 1;
            },
            _ => {
                i += 1;
            },
        }
    }
    map
}

#[allow(dead_code)]
fn show_hex_32(bytes: &Vec<u8>) {
    let mut ct = 0;
    for byte in bytes {
        print!("{:02X} ", byte);
        ct += 1;
        if ct % 16 == 0 {
            println!("")
        }
    }
    if !(ct % 16 == 0) {
        println!("")
    }
}

#[allow(dead_code)]
fn show_hex_64(bytes: &Vec<u8>) {
    let mut ct = 0;
    for byte in bytes {
        print!("{:02X} ", byte);
        ct += 1;
        if ct % 32 == 0 {
            println!("")
        }
    }
    if !(ct % 32 == 0) {
        println!("")
    }
}

fn main() {
    // read first arg as file to string
    let data: String = read_input_file();

    let mut code_txt: Vec<char> = data.chars()
                                      .collect();
    // filter out characters not present in COMMANDS
    code_txt.retain(|&c| COMMANDS.contains(&c));

    println!("TXT: {}", data);

    // parse into CST
    let parsed_code = parse(&mut code_txt);

    // this struct will create and store our code from the CST
    let mut buffer = Buff {
        data: vec![], // where our code is stored
        jmp_stack: vec![], // used for tracking jmp offsets
    };

    {
        // allocate runtime mem
        let mem: [u8; MAX_MEM] = [0; MAX_MEM];
        // encode our program alongside data for given memory region
        buffer.encode(parsed_code, mem.as_ptr());
        show_hex_32(&buffer.data);
        let program = ExecutableMemory::with_contents(&buffer.data);
        unsafe {
            let f = transmute::<*mut u8, unsafe fn()>(program.as_ptr());
            // let _blocker: [u128; 32] = [0; 32];
            f();
        }
        println!("Done");
    }
}

